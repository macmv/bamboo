use crate::{
  packet::Packet,
  packet_stream::{StreamReader, StreamWriter},
  version,
};

use common::{net::cb, proto, proto::minecraft_client::MinecraftClient, version::ProtocolVersion};
use std::{error::Error, io, io::ErrorKind, sync::Arc};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::channel::Channel, Request, Status, Streaming};

#[derive(Debug, Copy, Clone)]
pub enum State {
  Handshake,
  Status,
  Login,
  Play,
  Invalid,
}

impl State {
  fn from_next(next: i32) -> Self {
    if next == 1 {
      Self::Status
    } else if next == 2 {
      Self::Login
    } else {
      Self::Invalid
    }
  }
}

pub struct Conn {
  client_reader: StreamReader,
  client_writer: StreamWriter,
  server:        MinecraftClient<Channel>,
  state:         State,
  gen:           Arc<version::Generator>,
  ver:           ProtocolVersion,
}

pub struct ClientListener {
  client: StreamReader,
  server: mpsc::Sender<proto::Packet>,
  gen:    Arc<version::Generator>,
  ver:    ProtocolVersion,
}

pub struct ServerListener {
  client: StreamWriter,
  server: Streaming<proto::Packet>,
  gen:    Arc<version::Generator>,
  ver:    ProtocolVersion,
}

impl ClientListener {
  /// This starts listening for packets from the server. The rx and tx are used
  /// to close the ServerListener. Specifically, the tx will send a value once
  /// this listener has been closed, and this listener will close once the rx
  /// gets a message.
  pub async fn run(
    &mut self,
    tx: oneshot::Sender<()>,
    rx: oneshot::Receiver<()>,
  ) -> Result<(), Box<dyn Error>> {
    let res = self.run_inner(rx).await;
    // Close the other connection. We ignore the result, as that means the rx has
    // been dropped. We don't care if the rx has been dropped, because that means
    // the other listener has already closed.
    let _ = tx.send(());
    res
  }
  async fn run_inner(&mut self, mut rx: oneshot::Receiver<()>) -> Result<(), Box<dyn Error>> {
    loop {
      let fut = self.client.poll();

      tokio::select! {
        _ = fut => (),
        _ = &mut rx => break,
      }
      loop {
        let p = self.client.read(self.ver).unwrap();
        if p.is_none() {
          break;
        }
        let p = p.unwrap();
        let err = p.err();
        match err {
          Some(e) => {
            error!("error while parsing packet: {}", e);
            return Err(Box::new(io::Error::new(
              ErrorKind::InvalidData,
              format!("failed to parse packet, closing connection"),
            )));
          }
          None => {}
        }
        let sb = self.gen.serverbound(self.ver, p)?;
        info!("got proto: {}", &sb);
        self.server.send(sb.to_proto()).await?;
      }
    }
    Ok(())
  }
}

impl ServerListener {
  /// This starts listening for packets from the server. The rx and tx are used
  /// to close the ClientListener. Specifically, the tx will send a value once
  /// this listener has been closed, and this listener will close once the rx
  /// gets a message.
  pub async fn run(
    &mut self,
    tx: oneshot::Sender<()>,
    rx: oneshot::Receiver<()>,
  ) -> Result<(), Box<dyn Error>> {
    let res = self.run_inner(rx).await;
    let _ = tx.send(());
    res
  }
  async fn run_inner(&mut self, mut rx: oneshot::Receiver<()>) -> Result<(), Box<dyn Error>> {
    loop {
      let pb = self.server.message();
      let p;

      tokio::select! {
        val = pb => p = val?,
        _ = &mut rx => break,
      }
      let p = p.unwrap();
      let cb = self.gen.clientbound(self.ver, cb::Packet::from_proto(p))?;
      self.client.write(cb).await.unwrap();
    }
    Ok(())
  }
}

impl Conn {
  pub async fn new(
    client_reader: StreamReader,
    client_writer: StreamWriter,
    ip: String,
  ) -> Result<Self, tonic::transport::Error> {
    Ok(Conn {
      client_reader,
      client_writer,
      server: MinecraftClient::connect(ip).await?,
      state: State::Handshake,
      gen: Arc::new(version::Generator::new()),
      ver: ProtocolVersion::Invalid,
    })
  }

  pub async fn split(mut self) -> Result<(ClientListener, ServerListener), Status> {
    let (tx, rx) = mpsc::channel(8);

    let response = self.server.connection(Request::new(ReceiverStream::new(rx))).await?;
    let inbound = response.into_inner();

    Ok((
      ClientListener {
        ver:    self.ver,
        gen:    self.gen.clone(),
        client: self.client_reader,
        server: tx,
      },
      ServerListener {
        ver:    self.ver,
        gen:    self.gen,
        client: self.client_writer,
        server: inbound,
      },
    ))
  }

  pub async fn handshake(&mut self) -> io::Result<()> {
    'login: loop {
      self.client_reader.poll().await.unwrap();
      loop {
        let p = self.client_reader.read(self.ver).unwrap();
        if p.is_none() {
          break;
        }
        let mut p = p.unwrap();
        let err = p.err();
        match err {
          Some(e) => {
            error!("error while parsing packet: {}", e);
            break;
          }
          None => {}
        }
        match self.state {
          State::Handshake => {
            if p.id() != 0 {
              return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!("unknown handshake packet {}", p.id()),
              ));
            }
            self.ver = ProtocolVersion::from(p.read_varint());
            if self.ver == ProtocolVersion::Invalid {
              return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!("client sent an invalid version"),
              ));
            }

            let _addr = p.read_str();
            let _port = p.read_u16();
            let next = p.read_varint();
            self.state = State::from_next(next);
          }
          State::Status => {}
          State::Login => {
            match p.id() {
              // Login start
              0 => {
                let username = p.read_str();
                info!("got username {}", username);
                let mut out = Packet::new(2, self.ver);
                out.write_str("a0ebbc8d-e0b0-4c23-a965-efba61ff0ae8");
                out.write_str("macmv");
                self.client_writer.write(out).await?;

                self.state = State::Play;
                // Successful login, we can break now
                break 'login;

                // let mut out = Packet::new(1);
                // out.buf.write_i32(0); // EID 0
                // out.buf.write_u8(1); // Creative
                // out.buf.write_u8(0); // Overworld
                // out.buf.write_u8(1); // Difficulty
                // out.buf.write_u8(1); // Max players
                // out.buf.write_str("default"); // Level type
                // out.buf.write_bool(false); // Don't reduce debug info
                // self.client.write(out).await?;
              }
              // Encryption response
              1 => {}
              _ => {
                return Err(io::Error::new(
                  ErrorKind::InvalidInput,
                  format!("unknown login packet {}", p.id()),
                ));
              }
            }
          }
          v => {
            return Err(io::Error::new(
              ErrorKind::InvalidInput,
              format!("invalid connection state {:?}", v),
            ));
          }
        }
      }
    }
    Ok(())
  }
}

use crate::{
  packet::Packet,
  packet_stream::{StreamReader, StreamWriter},
  version,
};

use common::{net::cb, proto, proto::minecraft_client::MinecraftClient, version::ProtocolVersion};
use std::{error::Error, io, io::ErrorKind, sync::Arc};
use tokio::sync::mpsc;
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
}

pub struct ClientListener {
  client: StreamReader,
  server: mpsc::Sender<proto::Packet>,
  gen:    Arc<version::Generator>,
}

pub struct ServerListener {
  client: StreamWriter,
  server: Streaming<proto::Packet>,
  gen:    Arc<version::Generator>,
}

impl ClientListener {
  pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
    'running: loop {
      self.client.poll().await?;
      loop {
        let p = self.client.read().unwrap();
        if p.is_none() {
          break;
        }
        let p = p.unwrap();
        let err = p.err();
        match err {
          Some(e) => {
            error!("error while parsing packet: {}", e);
            break 'running Err(Box::new(io::Error::new(
              ErrorKind::InvalidData,
              format!("failed to parse packet, closing connection"),
            )));
          }
          None => {}
        }
        info!("got packet: {:?}", &p);
        let sb = self.gen.serverbound(ProtocolVersion::V1_8, p)?;
        info!("got proto: {:?}", &sb);
        self.server.send(sb.to_proto()).await?;
      }
    }
  }
}

impl ServerListener {
  pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
    loop {
      let p = self.server.message().await?;
      if p.is_none() {
        break;
      }
      let p = p.unwrap();
      let cb = self.gen.clientbound(ProtocolVersion::V1_8, cb::Packet::from_proto(p))?;
      self.client.write(cb).await.unwrap();
    }
    info!("closing connection with server");

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
    })
  }

  pub async fn split(mut self) -> Result<(ClientListener, ServerListener), Status> {
    let (tx, rx) = mpsc::channel(8);

    let response = self.server.connection(Request::new(ReceiverStream::new(rx))).await?;
    let inbound = response.into_inner();

    Ok((
      ClientListener { gen: self.gen.clone(), client: self.client_reader, server: tx },
      ServerListener { gen: self.gen, client: self.client_writer, server: inbound },
    ))
  }

  pub async fn handshake(&mut self) -> io::Result<()> {
    let mut ver = None;
    'login: loop {
      self.client_reader.poll().await.unwrap();
      loop {
        let p = self.client_reader.read().unwrap();
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
            ver = Some(ProtocolVersion::from(p.read_varint()));
            // Make sure that the version is know to the reader/writers
            self.client_reader.ver = ver.unwrap();
            self.client_writer.ver = ver.unwrap();

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
                let mut out = Packet::new(2, ver.unwrap());
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

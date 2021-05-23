use crate::{
  packet::Packet,
  packet_stream::{StreamReader, StreamWriter},
  version::Generator,
};

use common::{
  math::UUID,
  net::{cb, sb},
  proto,
  proto::minecraft_client::MinecraftClient,
  util::{chat::HoverEvent, Chat},
  version::ProtocolVersion,
};
use serde_derive::Serialize;
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
  gen:           Arc<Generator>,
  ver:           ProtocolVersion,
}

pub struct ClientListener {
  client: StreamReader,
  server: mpsc::Sender<proto::Packet>,
  gen:    Arc<Generator>,
  ver:    ProtocolVersion,
}

pub struct ServerListener {
  client: StreamWriter,
  server: Streaming<proto::Packet>,
  gen:    Arc<Generator>,
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
      tokio::select! {
        v = self.client.poll() => v?,
        _ = &mut rx => break,
      }
      loop {
        let p = self.client.read(self.ver).unwrap();
        let p = if let Some(v) = p { v } else { break };
        // Make sure there were no errors set within the packet during parsing
        match p.err() {
          Some(e) => {
            error!("error while parsing packet: {}", e);
            return Err(Box::new(io::Error::new(
              ErrorKind::InvalidData,
              "failed to parse packet, closing connection",
            )));
          }
          None => {}
        }
        // Converting a tcp packet to a grpc packet should always work. If it fails,
        // then it is either an invalid version, an unknown packet, or a parsing error.
        // If it is a parsing error, we want to close the connection.
        let sb = match self.gen.serverbound(self.ver, p) {
          Err(e) => match e.kind() {
            ErrorKind::Other => {
              return Err(Box::new(e));
            }
            _ => {
              warn!("{}", e);
              continue;
            }
          },
          Ok(v) => v,
        };
        trace!("sending proto: {}", &sb);
        self.server.send(sb.into_proto()).await?;
      }
    }
    Ok(())
  }

  /// Sends a packet to the server. Should only be used for things like login
  /// packets.
  pub async fn send_to_server(&mut self, p: sb::Packet) -> Result<(), Box<dyn Error>> {
    self.server.send(p.into_proto()).await?;
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
        v = pb => p = v?,
        _ = &mut rx => break,
      }
      let p = p.unwrap();
      let cb = self.gen.clientbound(self.ver, cb::Packet::from_proto(p))?;
      if let Some(p) = cb {
        self.client.write(p).await?;
      }
    }
    Ok(())
  }
}

#[derive(Serialize)]
struct JsonStatus {
  version:     JsonVersion,
  players:     JsonPlayers,
  description: Chat,
  favicon:     String,
}

#[derive(Serialize)]
struct JsonVersion {
  name:     String,
  protocol: i32,
}

#[derive(Serialize)]
struct JsonPlayers {
  max:    i32,
  online: i32,
  sample: Vec<JsonPlayer>,
}

#[derive(Serialize)]
struct JsonPlayer {
  name: String,
  id:   String,
}

impl Conn {
  pub async fn new(
    gen: Arc<Generator>,
    client_reader: StreamReader,
    client_writer: StreamWriter,
    ip: String,
  ) -> Result<Self, tonic::transport::Error> {
    Ok(Conn {
      client_reader,
      client_writer,
      server: MinecraftClient::connect(ip).await?,
      state: State::Handshake,
      gen,
      ver: ProtocolVersion::Invalid,
    })
  }

  pub async fn split(mut self) -> Result<(ClientListener, ServerListener), Status> {
    let (tx, rx) = mpsc::channel(1);

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

  pub async fn handshake(
    &mut self,
    compression: i32,
    der_key: Option<&[u8]>,
  ) -> io::Result<(String, UUID)> {
    let mut username = None;
    let mut uuid;
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
                "client sent an invalid version",
              ));
            }

            let _addr = p.read_str();
            let _port = p.read_u16();
            let next = p.read_varint();
            self.state = State::from_next(next);
          }
          State::Status => {
            match p.id() {
              // Server status
              0 => {
                let mut description = Chat::empty();
                description.add("Big ".into());
                description
                  .add("Gaming".into())
                  .on_hover(HoverEvent::ShowText("I am Gaming".into()));
                let status = JsonStatus {
                  version: JsonVersion { name: "1.8".into(), protocol: self.ver.id() as i32 },
                  players: JsonPlayers {
                    max:    69,
                    online: 420,
                    sample: vec![JsonPlayer {
                      name: "macmv".into(),
                      id:   "a0ebbc8d-e0b0-4c23-a965-efba61ff0ae8".into(),
                    }],
                  },
                  description,
                  favicon: "".into(),
                };
                let mut out = Packet::new(0, self.ver);
                out.write_str(&serde_json::to_string(&status).unwrap());
                self.client_writer.write(out).await?;
              }
              _ => {
                return Err(io::Error::new(
                  ErrorKind::InvalidInput,
                  format!("unknown status packet {}", p.id()),
                ));
              }
            }
          }
          State::Login => {
            match p.id() {
              // Login start
              0 => {
                if username.is_some() {
                  return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "client sent two login packets",
                  ));
                }
                let name = p.read_str();
                info!("got username {}", &name);
                let id = UUID::from_bytes(*md5::compute(&name));

                username = Some(name);
                uuid = Some(id);

                match der_key {
                  Some(key) => {
                    // Encryption request
                    let mut out = Packet::new(1, self.ver);
                    out.write_str(""); // Server id, should be empty
                    out.write_varint(key.len() as i32); // Key len
                    out.write_buf(key); // DER encoded RSA key
                    self.client_writer.write(out).await?;
                    // Wait for encryption response to enable encryption
                  }
                  None => {
                    // Set compression, only if the thresh hold is non-zero
                    if compression != 0 {
                      let mut out = Packet::new(3, self.ver);
                      out.write_varint(compression);
                      self.client_writer.write(out).await?;
                      // Must happen after the packet has been sent
                      self.client_writer.set_compression(compression);
                      self.client_reader.set_compression(compression);
                    }

                    // Login success
                    let mut out = Packet::new(2, self.ver);
                    out.write_str(&id.as_dashed_str());
                    out.write_str(username.as_ref().unwrap());
                    self.client_writer.write(out).await?;

                    self.state = State::Play;
                    // Successful login, we can break now
                    break 'login;
                  }
                }

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
    Ok((username.unwrap(), uuid.unwrap()))
  }
}

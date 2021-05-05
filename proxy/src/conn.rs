use crate::{
  packet::Packet,
  packet_stream::{StreamReader, StreamWriter},
};

use common::{proto, proto::minecraft_client::MinecraftClient};
use std::{error::Error, io, io::ErrorKind};
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
}

pub struct ClientListener {
  client: StreamReader,
  server: mpsc::Sender<proto::Packet>,
}

pub struct ServerListener {
  client: StreamWriter,
  server: Streaming<proto::Packet>,
}

impl ClientListener {
  pub async fn run(&mut self) -> io::Result<()> {
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
            break 'running Err(io::Error::new(
              ErrorKind::InvalidInput,
              format!("failed to parse packet, closing connection"),
            ));
          }
          None => {}
        }
        info!("got tcp packet {:?}", p);
      }
    }
  }
}

impl ServerListener {
  pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
    loop {
      match self.server.message().await? {
        Some(m) => {
          info!("got grpc packet {:?}", m);
        }
        None => break,
      }
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
    })
  }

  pub async fn split(mut self) -> Result<(ClientListener, ServerListener), Status> {
    let (tx, rx) = mpsc::channel(8);

    let response = self.server.connection(Request::new(ReceiverStream::new(rx))).await?;
    let inbound = response.into_inner();

    Ok((
      ClientListener { client: self.client_reader, server: tx },
      ServerListener { client: self.client_writer, server: inbound },
    ))
  }

  pub async fn handshake(&mut self) -> io::Result<()> {
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
            let _version = p.buf.read_varint();
            let _addr = p.buf.read_str();
            let _port = p.buf.read_u16();
            let next = p.buf.read_varint();
            self.state = State::from_next(next);
          }
          State::Status => {}
          State::Login => {
            match p.id() {
              // Login start
              0 => {
                let username = p.buf.read_str();
                info!("got username {}", username);
                let mut out = Packet::new(2);
                out.buf.write_str("a0ebbc8d-e0b0-4c23-a965-efba61ff0ae8");
                out.buf.write_str("macmv");
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

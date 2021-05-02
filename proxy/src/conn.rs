use crate::{packet::Packet, packet_stream::Stream};

use common::proto::minecraft_client::MinecraftClient;
use std::{
  io,
  io::{Error, ErrorKind},
};
use tonic::transport::channel::Channel;

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
  client: Stream,
  server: MinecraftClient<Channel>,
  state: State,
}

impl Conn {
  pub async fn new(client: Stream, ip: String) -> Result<Self, tonic::transport::Error> {
    Ok(Conn { client, server: MinecraftClient::connect(ip).await?, state: State::Handshake })
  }

  pub async fn handshake(&mut self) -> io::Result<()> {
    'login: loop {
      self.client.poll().await.unwrap();
      loop {
        let p = self.client.read().unwrap();
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
              return Err(Error::new(
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
                self.client.write(out).await?;

                self.state = State::Play;
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
                return Err(Error::new(
                  ErrorKind::InvalidInput,
                  format!("unknown login packet {}", p.id()),
                ));
              }
            }
          }
          v => {
            return Err(Error::new(
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

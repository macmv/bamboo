#[macro_use]
extern crate log;

pub mod packet;
pub mod packet_stream;

use crate::packet_stream::Stream;

use common::{
  proto::{
    minecraft_client::MinecraftClient, Packet, ReserveSlotsRequest, ReserveSlotsResponse,
    StatusRequest, StatusResponse,
  },
  util,
};
use std::error::Error;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  common::init();

  let addr = "0.0.0.0:25565";
  info!("listening for clients on {}", addr);
  let listener = TcpListener::bind(addr).await?;

  loop {
    let (socket, _) = listener.accept().await?;
    tokio::spawn(async move {
      handle_client(socket).await.unwrap();
    });
  }
}

async fn handle_client(sock: TcpStream) -> Result<(), Box<dyn Error>> {
  // let mut client = MinecraftClient::connect("http://0.0.0.0:8483").await?;
  // let req = tonic::Request::new(StatusRequest {});

  let mut stream = Stream::new(sock);

  // TODO: Move this into an enum
  let mut state = 0;

  'login: loop {
    stream.poll().await.unwrap();
    loop {
      let p = stream.read().unwrap();
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
      match state {
        // Handshake
        0 => {
          if p.id() != 0 {
            error!("unknown handshake packet id: {}", p.id());
            break 'login;
          }
          let _version = p.buf.read_varint();
          let _addr = p.buf.read_str();
          let _port = p.buf.read_u16();
          let next = p.buf.read_varint();
          state = next;
        }
        // Status
        1 => {}
        // Login
        2 => {
          match p.id() {
            // Login start
            0 => {
              let username = p.buf.read_str();
              info!("got username {}", username);
            }
            // Encryption response
            1 => {}
            _ => {
              error!("unknown handshake packet id: {}", p.id());
              break 'login;
            }
          }
        }
        // Play
        3 => {}
        _ => {
          error!("invalid state: {}", state);
          break 'login;
        }
      }
    }
  }

  // info!("New client!");
  // let res = client.status(req).await?;
  //
  // dbg!(res);

  Ok(())
}

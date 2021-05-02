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
  info!("Listening for clients on {}", addr);
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

  loop {
    stream.poll().await.unwrap();
    loop {
      let p = stream.read().unwrap();
      if p.is_none() {
        break;
      }
      let p = p.unwrap();
      let err = p.err();
      match err {
        Some(e) => {
          error!("error while parsing packet: {}", e);
          break;
        }
        None => {}
      }
      info!("Got minecraft packet: {:?}", p);
    }
    break;
  }

  // info!("New client!");
  // let res = client.status(req).await?;
  //
  // dbg!(res);

  Ok(())
}

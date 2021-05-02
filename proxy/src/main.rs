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

  let mut buf = util::Buffer::new(vec![1, 2, 3]);
  let a = buf.read_u8();
  let b = buf.read_u16();
  dbg!(a, b, buf);

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
  let mut client = MinecraftClient::connect("http://0.0.0.0:8483").await?;

  let req = tonic::Request::new(StatusRequest {});

  let stream = Stream::new(sock);

  loop {
    let p = stream.read();
    info!("Got minecraft packet: {:?}", p);
    break;
  }

  info!("New client!");
  let res = client.status(req).await?;

  dbg!(res);

  Ok(())
}

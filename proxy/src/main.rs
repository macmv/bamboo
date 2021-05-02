#[macro_use]
extern crate log;

pub mod conn;
pub mod packet;
pub mod packet_stream;

use crate::{conn::Conn, packet::Packet, packet_stream::Stream};

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
  // let mut client = MinecraftClient::connect().await?;
  // let req = tonic::Request::new(StatusRequest {});

  let stream = Stream::new(sock);
  let mut conn = Conn::new(stream, "http://0.0.0.0:8483".into()).await;

  conn.handshake().await?;

  // info!("New client!");
  // let res = client.status(req).await?;
  //
  // dbg!(res);

  Ok(())
}

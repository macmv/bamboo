#[macro_use]
extern crate log;

use common::proto::{
  minecraft_client::MinecraftClient, Packet, ReserveSlotsRequest, ReserveSlotsResponse,
  StatusRequest, StatusResponse,
};
use std::error::Error;
use tokio::net::TcpListener;

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

async fn handle_client<T>(sock: T) -> Result<(), Box<dyn Error>> {
  let mut client = MinecraftClient::connect("http://0.0.0.0:8483").await?;

  let req = tonic::Request::new(StatusRequest {});

  println!("Sending request to gRPC Server...");
  let res = client.status(req).await?;

  dbg!(res);

  Ok(())
}

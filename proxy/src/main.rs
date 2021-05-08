#![feature(trait_alias)]

#[macro_use]
extern crate log;

pub mod conn;
pub mod packet;
pub mod packet_stream;
pub mod version;

use std::{
  error::Error,
  net::{TcpListener, TcpStream},
};
use tokio::sync::oneshot;

use crate::conn::Conn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  common::init();

  let addr = "0.0.0.0:25565";
  info!("listening for clients on {}", addr);
  let listener = TcpListener::bind(addr)?;

  loop {
    let (socket, _) = listener.accept()?;
    tokio::spawn(async move {
      match handle_client(socket).await {
        Ok(_) => {}
        Err(e) => {
          error!("error in connection: {}", e);
        }
      };
    });
  }
}

async fn handle_client(sock: TcpStream) -> Result<(), Box<dyn Error>> {
  // let mut client = MinecraftClient::connect().await?;
  // let req = tonic::Request::new(StatusRequest {});

  let (reader, writer) = packet_stream::new(sock)?;
  let mut conn = Conn::new(reader, writer, "http://0.0.0.0:8483".into()).await?;

  conn.handshake().await?;

  // These four values are passed to each listener. When one listener closes, it
  // sends a message to the tx. Since the rx is passed to the other listener, that
  // listener will then close itself.
  let (server_tx, client_rx) = oneshot::channel();
  let (client_tx, server_rx) = oneshot::channel();

  let (mut client_listener, mut server_listener) = conn.split().await?;
  let mut handles = vec![];
  handles.push(tokio::spawn(async move {
    match client_listener.run(client_tx, client_rx).await {
      Ok(_) => {}
      Err(e) => {
        error!("error while listening to client: {}", e);
      }
    };
  }));
  handles.push(tokio::spawn(async move {
    match server_listener.run(server_tx, server_rx).await {
      Ok(_) => {}
      Err(e) => {
        error!("error while listening to server: {}", e);
      }
    };
  }));

  futures::future::join_all(handles).await;

  info!("All tasks have closed!");

  // info!("New client!");
  // let res = client.status(req).await?;
  //
  // dbg!(res);

  Ok(())
}

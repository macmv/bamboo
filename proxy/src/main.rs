#[macro_use]
extern crate log;

pub mod conn;
pub mod packet;
pub mod packet_stream;
pub mod version;

use rand::rngs::OsRng;
use rsa::{PublicKeyParts, RSAPrivateKey};
use std::{
  error::Error,
  net::{TcpListener, TcpStream},
  sync::Arc,
};
use tokio::sync::oneshot;

use crate::conn::Conn;
use common::net::sb;
use version::Generator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  common::init();

  let addr = "0.0.0.0:25565";
  info!("listening for clients on {}", addr);
  let listener = TcpListener::bind(addr)?;
  let gen = Arc::new(Generator::new());

  // Minecraft uses 1024 bits for this.
  let key = RSAPrivateKey::new(&mut OsRng, 1024).expect("failed to generate a key");
  let der_key = Some(rsa_der::public_key_to_der(&key.n().to_bytes_be(), &key.e().to_bytes_be()));

  loop {
    let (socket, _) = listener.accept()?;
    let gen = gen.clone();
    let k = key.clone();
    let d = der_key.clone();
    tokio::spawn(async move {
      match handle_client(gen, socket, k, d).await {
        Ok(_) => {}
        Err(e) => {
          error!("error in connection: {}", e);
        }
      };
    });
  }
}

async fn handle_client(
  gen: Arc<Generator>,
  sock: TcpStream,
  key: RSAPrivateKey,
  der_key: Option<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
  // let mut client = MinecraftClient::connect().await?;
  // let req = tonic::Request::new(StatusRequest {});

  let (reader, writer) = packet_stream::new(sock)?;
  let mut conn = Conn::new(gen, reader, writer, "http://0.0.0.0:8483".into()).await?;

  let compression = 256;

  let (name, id) = conn.handshake(compression, key, der_key).await?;

  // These four values are passed to each listener. When one listener closes, it
  // sends a message to the tx. Since the rx is passed to the other listener, that
  // listener will then close itself.
  let (server_tx, client_rx) = oneshot::channel();
  let (client_tx, server_rx) = oneshot::channel();

  let (mut client_listener, mut server_listener) = conn.split().await?;

  // Tells the server who this client is
  let mut out = sb::Packet::new(sb::ID::Login);
  out.set_str("username".into(), name);
  out.set_uuid("uuid".into(), id);
  client_listener.send_to_server(out).await?;

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

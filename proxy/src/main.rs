#[macro_use]
extern crate log;

pub mod conn;
pub mod version;

use rand::rngs::OsRng;
use rsa::RSAPrivateKey;
use std::{error::Error, sync::Arc};
use tokio::{net::TcpListener, sync::oneshot};

use crate::conn::Conn;
use common::{
  math::der,
  net::sb,
  stream::{bedrock, java, StreamReader, StreamWriter},
};
use version::Generator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  common::init("proxy");

  let addr = "0.0.0.0:25565";
  info!("listening for java clients on {}", addr);
  let java_listener = TcpListener::bind(addr).await?;

  let addr = "0.0.0.0:19132";
  info!("listening for bedrock clients on {}", addr);
  let mut bedrock_listener = bedrock::Listener::bind(addr).await?;

  let gen = Arc::new(Generator::new());

  // Minecraft uses 1024 bits for this.
  let key = RSAPrivateKey::new(&mut OsRng, 1024).expect("failed to generate a key");
  let der_key = Some(der::encode(&key));

  let gen2 = gen.clone();
  let key2 = key.clone();
  let java_handle = tokio::spawn(async move {
    loop {
      let (sock, _) = java_listener.accept().await.unwrap();
      let (reader, writer) = java::stream::new(sock).unwrap();
      let gen = gen.clone();
      let k = key.clone();
      let _d = der_key.clone();
      tokio::spawn(async move {
        match handle_client(gen, reader, writer, k, None).await {
          Ok(_) => {}
          Err(e) => {
            error!("error in connection: {}", e);
          }
        };
      });
    }
  });
  let bedrock_handle = tokio::spawn(async move {
    loop {
      if let Some((reader, writer)) = bedrock_listener.poll().await.unwrap() {
        let gen = gen2.clone();
        let k = key2.clone();
        tokio::spawn(async move {
          match handle_client(gen, reader, writer, k, None).await {
            Ok(_) => {}
            Err(e) => {
              error!("error in connection: {}", e);
            }
          };
        });
      }
    }
  });
  futures::future::join_all(vec![java_handle, bedrock_handle]).await;
  Ok(())
}

async fn handle_client<R: StreamReader + Send + 'static, W: StreamWriter + Send + 'static>(
  gen: Arc<Generator>,
  reader: R,
  writer: W,
  key: RSAPrivateKey,
  der_key: Option<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
  // let mut client = MinecraftClient::connect().await?;
  // let req = tonic::Request::new(StatusRequest {});

  let mut conn = Conn::new(gen, reader, writer, "http://0.0.0.0:8483".into()).await?;

  let compression = 256;

  let info = match conn.handshake(compression, key, der_key).await? {
    Some(v) => v,
    // Means the client was either not allowed to join, or was just sending a status request.
    None => return Ok(()),
  };

  // These four values are passed to each listener. When one listener closes, it
  // sends a message to the tx. Since the rx is passed to the other listener, that
  // listener will then close itself.
  let (server_tx, client_rx) = oneshot::channel();
  let (client_tx, server_rx) = oneshot::channel();

  let ver = conn.ver().id() as i32;
  let (mut client_listener, mut server_listener) = conn.split().await?;

  // Tells the server who this client is
  // TODO: Send texture data
  client_listener
    .send_to_server(sb::Packet::Login { username: info.name, uuid: info.id, ver })
    .await?;

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

#[macro_use]
extern crate log;

#[macro_use]
extern crate async_trait;

pub mod bedrock;
pub mod conn;
pub mod java;
pub mod packet;
pub mod version;

use common::version::ProtocolVersion;
use rand::rngs::OsRng;
use rsa::{PublicKeyParts, RSAPrivateKey};
use std::{
  error::Error,
  io,
  net::{TcpListener, TcpStream},
  sync::Arc,
};
use tokio::sync::oneshot;

use crate::conn::Conn;
use common::net::sb;
use packet::Packet;
use version::Generator;

#[async_trait]
pub trait StreamReader {
  async fn poll(&mut self) -> io::Result<()> {
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> io::Result<Option<Packet>>;

  fn enable_encryption(&mut self, secret: &[u8; 16]) {}
  fn set_compression(&mut self, level: i32) {}
}
#[async_trait]
pub trait StreamWriter {
  async fn write(&mut self, packet: Packet) -> io::Result<()>;

  fn enable_encryption(&mut self, secret: &[u8; 16]) {}
  fn set_compression(&mut self, level: i32) {}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  common::init("proxy");

  let addr = "0.0.0.0:25565";
  info!("listening for java clients on {}", addr);
  let tcp_listener = TcpListener::bind(addr)?;

  let addr = "0.0.0.0:19132";
  info!("listening for bedrock clients on {}", addr);
  let raknet_listener = bedrock::Listener::bind(addr)?;

  let gen = Arc::new(Generator::new());

  // Minecraft uses 1024 bits for this.
  let key = RSAPrivateKey::new(&mut OsRng, 1024).expect("failed to generate a key");
  let der_key = Some(rsa_der::public_key_to_der(&key.n().to_bytes_be(), &key.e().to_bytes_be()));

  let gen2 = gen.clone();
  let key2 = key.clone();
  let tcp_handle = tokio::spawn(async move {
    loop {
      let (sock, _) = tcp_listener.accept().unwrap();
      let (reader, writer) = java::stream::new(sock).unwrap();
      let gen = gen.clone();
      let k = key.clone();
      let d = der_key.clone();
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
  let raknet_handle = tokio::spawn(async move {
    loop {
      let (reader, writer) = raknet_listener.accept().unwrap();
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
  });
  futures::future::join_all(vec![tcp_handle, raknet_handle]).await;
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
  let mut out = sb::Packet::new(sb::ID::Login);
  out.set_str("username".into(), info.name);
  out.set_uuid("uuid".into(), info.id);
  out.set_int("ver".into(), ver);
  // TODO: Send texture data
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

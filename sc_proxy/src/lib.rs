#[macro_use]
extern crate log;

pub mod conn;
pub mod stream;

use mio::{net::TcpListener, Events, Interest, Poll, Token, Waker};
use rand::rngs::OsRng;
use rsa::RSAPrivateKey;
use std::{collections::HashMap, error::Error, io, sync::Arc};

use crate::{
  conn::{Conn, ServerListener},
  stream::{java::stream::JavaStream, PacketStream},
};
use sc_common::proto::minecraft_client::MinecraftClient;

pub fn load_icon(path: &str) -> String {
  let mut icon = match image::open(path).map_err(|e| error!("error loading icon: {}", e)) {
    Ok(icon) => icon,
    Err(_) => return "".into(),
  };
  icon = icon.resize_exact(64, 64, image::imageops::FilterType::Triangle);
  let mut enc = base64::write::EncoderStringWriter::new(base64::STANDARD);
  icon.write_to(&mut enc, image::ImageFormat::Png).unwrap();
  "data:image/png;base64,".to_string() + &enc.into_inner()
}

pub async fn run() -> Result<(), Box<dyn Error>> {
  sc_common::init("proxy");

  const JAVA_LISTENER: Token = Token(0xffffffff);
  const BEDROCK_LISTENER: Token = Token(0xfffffffe);

  let addr = "0.0.0.0:25565";
  info!("listening for java clients on {}", addr);
  let mut java_listener = TcpListener::bind(addr.parse()?)?;

  // let addr = "0.0.0.0:19132";
  // info!("listening for bedrock clients on {}", addr);
  // let mut bedrock_listener = bedrock::Listener::bind(addr).await?;

  // Minecraft uses 1024 bits for this.
  let key = Arc::new(RSAPrivateKey::new(&mut OsRng, 1024).expect("failed to generate a key"));
  // let der_key = Some(Arc::new(der::encode(&key)));
  let der_key = None;
  let icon = Arc::new(load_icon("icon.png"));

  let mut poll = Poll::new()?;
  let mut events = Events::with_capacity(1024);
  let mut clients = HashMap::new();

  poll.registry().register(&mut java_listener, JAVA_LISTENER, Interest::READABLE)?;

  let mut next_token = 0;

  loop {
    // Wait for events
    poll.poll(&mut events, None)?;

    for event in &events {
      match event.token() {
        JAVA_LISTENER => {
          loop {
            match java_listener.accept() {
              Ok((mut client, _)) => {
                // New token for this client
                let token = Token(next_token);
                next_token += 1;

                // Register this client for events
                poll.registry().register(
                  &mut client,
                  token,
                  Interest::READABLE | Interest::WRITABLE,
                )?;
                clients.insert(
                  token,
                  new_conn(
                    JavaStream::new(client),
                    key.clone(),
                    der_key.clone(),
                    Waker::new(poll.registry(), token)?,
                    &icon,
                  )?,
                );
              }
              Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Socket is not ready anymore, stop accepting
                break;
              }
              Err(e) => error!("error while listening: {}", e),
            }
          }
        }
        BEDROCK_LISTENER => {
          unimplemented!();
        }
        token => {
          let conn = clients.get_mut(&token).expect("client doesn't exist!");
          loop {
            match conn.poll() {
              Ok(_) => match conn.read().await {
                Ok(_) => {
                  if conn.closed() {
                    clients.remove(&token);
                    break;
                  }
                }
                Err(e) => {
                  error!("error while parsing packet from client {:?}: {}", token, e);
                  clients.remove(&token);
                  break;
                }
              },
              Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Socket is not ready anymore, stop reading
                break;
              }
              Err(e) => {
                error!("error while listening to client {:?}: {}", token, e);
                clients.remove(&token);
                break;
              }
            }
          }
        }
      }
    }
  }
}

pub fn new_conn<'a, S: PacketStream + Send + Sync + 'static>(
  stream: S,
  key: Arc<RSAPrivateKey>,
  der_key: Option<Vec<u8>>,
  waker: Waker,
  icon: &str,
) -> Result<Conn<S>, Box<dyn Error>> {
  let ip = "http://0.0.0.0:8483".to_string();
  let compression = 256;

  let client = futures::executor::block_on(MinecraftClient::connect(ip))?;
  let (conn, mut server_listener) = Conn::new(stream, client, compression, key, der_key, icon)?;

  tokio::spawn(async move {
    match server_listener.run(waker).await {
      Ok(_) => {}
      Err(e) => {
        error!("error while listening to client: {}", e);
      }
    };
  });
  Ok(conn)
}

pub fn handle_client<'a, S: PacketStream + Send + Sync + 'static>(
  stream: S,
  key: Arc<RSAPrivateKey>,
  der_key: Option<Arc<Vec<u8>>>,
  icon: &'a str,
) -> Result<(), Box<dyn Error>> {
  // let req = tonic::Request::new(StatusRequest {});

  let ip = "http://0.0.0.0:8483".to_string();
  let compression = 256;

  // let client = futures::executor::block_on(MinecraftClient::connect(ip))?;
  //
  // let mut conn = Conn::new(stream, client, icon)?;
  // let info = match conn.handshake(compression, key, der_key)? {
  //   Some(v) => v,
  //   // Means the client was either not allowed to join, or was just sending a
  // status request.   None => return Ok(()),
  // };

  // These four values are passed to each listener. When one listener closes, it
  // sends a message to the tx. Since the rx is passed to the other listener, that
  // listener will then close itself.
  // let (server_tx, client_rx) = oneshot::channel();
  // let (client_tx, server_rx) = oneshot::channel();

  // let ver = conn.ver().id() as i32;
  // let (mut client_listener, mut server_listener) = conn.split(ip)?;

  // Tells the server who this client is
  // TODO: Send texture data
  // client_listener.send_to_server(sb::Packet::Login { username: info.name, uuid:
  // info.id, ver })?;

  // let mut handles = vec![];
  // handles.push(tokio::spawn(async move {
  //   match client_listener.run(client_tx, client_rx).await {
  //     Ok(_) => {}
  //     Err(e) => {
  //       error!("error while listening to client: {}", e);
  //     }
  //   };
  // }));
  // handles.push(tokio::spawn(async move {
  //   match server_listener.run(server_tx, server_rx).await {
  //     Ok(_) => {}
  //     Err(e) => {
  //       error!("error while listening to server: {}", e);
  //     }
  //   };
  // }));
  //
  // futures::future::join_all(handles);

  info!("All tasks have closed!");

  // info!("New client!");
  // let res = client.status(req).await?;
  //
  // dbg!(res);

  Ok(())
}

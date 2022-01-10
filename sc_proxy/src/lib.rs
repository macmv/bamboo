#[macro_use]
extern crate log;

pub mod conn;
pub mod gnet;
pub mod packet;
pub mod stream;

use mio::{net::TcpListener, Events, Interest, Poll, Token};
use rand::rngs::OsRng;
use rsa::RSAPrivateKey;
use std::{collections::HashMap, error::Error, io, net::SocketAddr, sync::Arc};

use crate::{conn::Conn, packet::TypeConverter, stream::java::stream::JavaStream};

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

pub fn run() -> Result<(), Box<dyn Error>> {
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
  let server_ip: SocketAddr = "0.0.0.0:8483".parse().unwrap();
  let compression = 256;

  let mut poll = Poll::new()?;
  let mut events = Events::with_capacity(1024);
  let mut clients = HashMap::new();

  poll.registry().register(&mut java_listener, JAVA_LISTENER, Interest::READABLE)?;

  let mut next_token = 0;

  let conv = Arc::new(TypeConverter::new());

  loop {
    // Wait for events
    poll.poll(&mut events, None)?;

    for event in &events {
      match event.token() {
        JAVA_LISTENER => {
          loop {
            match java_listener.accept() {
              Ok((mut client, _)) => {
                // This is the tcp stream connected to the client
                let client_token = Token(next_token);
                // This is the tcp stream connected to the server
                let server_token = Token(next_token + 1);
                next_token += 2;

                // Register this client for events
                poll.registry().register(
                  &mut client,
                  client_token,
                  Interest::READABLE | Interest::WRITABLE,
                )?;
                // We will register the server tcp connection later, once we are done
                // handshaking.
                clients.insert(
                  client_token,
                  Conn::new(
                    JavaStream::new(client),
                    server_ip.clone(),
                    compression,
                    key.clone(),
                    der_key.clone(),
                    &icon,
                    server_token,
                    conv.clone(),
                  ),
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
          let is_server = token.0 % 2 != 0;
          let token = Token(token.0 / 2 * 2);

          if is_server {
            if event.is_readable() {
              let conn = clients.get_mut(&token).expect("client doesn't exist!");
              match conn.read_server() {
                Ok(_) => {}
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => {
                  error!("error while parsing packet from server {:?}: {}", token, e);
                  clients.remove(&token);
                }
              }
            }

            if event.is_writable() {
              if let Some(conn) = clients.get_mut(&token) {
                match conn.write_server() {
                  Ok(_) => {}
                  Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                  Err(e) => {
                    error!("error while flushing packets to the client {:?}: {}", token, e);
                    clients.remove(&token);
                  }
                }
              }
            }
          } else {
            if event.is_readable() {
              let conn = clients.get_mut(&token).expect("client doesn't exist!");
              match conn.read_client(&poll.registry()) {
                Ok(false) => {}
                Ok(true) => {
                  clients.remove(&token);
                }
                Err(e) => {
                  error!("error while parsing packet from client {:?}: {}", token, e);
                  clients.remove(&token);
                }
              }
            }
            // The order here is important. If we are handshaking, then reading a packet
            // will probably prompt a direct response. In this arrangement, we can send more
            // packets before going back to poll().
            if event.is_writable() {
              if let Some(conn) = clients.get_mut(&token) {
                match conn.write_client() {
                  Ok(_) => {}
                  Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                  Err(e) => {
                    error!("error while flushing packets to the client {:?}: {}", token, e);
                    clients.remove(&token);
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}

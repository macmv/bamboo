#[macro_use]
extern crate log;

pub mod conn;
pub mod stream;

use crossbeam_channel::TryRecvError;
use mio::{net::TcpListener, Events, Interest, Poll, Token, Waker};
use rand::rngs::OsRng;
use rsa::RSAPrivateKey;
use std::{collections::HashMap, error::Error, io, sync::Arc};
use tonic::transport::Endpoint;

use crate::{conn::Conn, stream::java::stream::JavaStream};

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
  const WAKE_TOKEN: Token = Token(0xfffffffd);

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
  let server_ip: Endpoint = "http://0.0.0.0:8483".parse().unwrap();
  let compression = 256;

  let mut poll = Poll::new()?;
  let mut events = Events::with_capacity(1024);
  let waker = Arc::new(Waker::new(poll.registry(), WAKE_TOKEN)?);
  let mut clients = HashMap::new();
  let (needs_flush_tx, needs_flush_rx) = crossbeam_channel::bounded(1024);

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
                    client_token,
                    server_token,
                    waker.clone(),
                    needs_flush_tx.clone(),
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
        WAKE_TOKEN => loop {
          match needs_flush_rx.try_recv() {
            Ok(token) => {
              let conn = match clients.get_mut(&token) {
                Some(conn) => conn,
                // Old message, before the connection was closed, so we ignore it.
                None => continue,
              };
              let mut wrote = false;
              while conn.needs_send() {
                wrote = true;
                match conn.write() {
                  Ok(_) => {}
                  Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Happens if the recieving stream is empty.
                    break;
                  }
                  Err(e) => {
                    error!("error while listening to server {:?}: {}", token, e);
                    clients.remove(&token);
                    break;
                  }
                }
              }
              if wrote {
                let conn = clients.get_mut(&token).expect("client doesn't exist!");
                while conn.needs_flush() {
                  match conn.flush() {
                    Ok(_) => {}
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => {
                      error!("error while flushing packets to the client {:?}: {}", token, e);
                      clients.remove(&token);
                      break;
                    }
                  }
                }
              }
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => unreachable!("needs_flush channel closed"),
          }
        },
        token => {
          let is_server = token.0 % 2 != 0;
          let token = Token(token.0 / 2 * 2);

          let mut closed = false;
          if is_server {
            if event.is_readable() {
              let conn = clients.get_mut(&token).expect("client doesn't exist!");
            }
          } else {
            if event.is_readable() {
              let conn = clients.get_mut(&token).expect("client doesn't exist!");
              match conn.read_all() {
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
            if event.is_writable() && !closed {
              let conn = clients.get_mut(&token).expect("client doesn't exist!");
              while conn.needs_flush() {
                match conn.flush() {
                  Ok(_) => {}
                  Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                  Err(e) => {
                    error!("error while flushing packets to the client {:?}: {}", token, e);
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
  }
}

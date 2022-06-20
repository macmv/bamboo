#![allow(clippy::identity_op)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate smallvec;

pub mod conn;
mod error;
pub mod gnet;
pub mod packet;
pub mod stream;

pub use error::{Error, Result};

use bb_common::{config::Config, math::der};
use mio::{
  event::Event,
  net::{TcpListener, TcpStream},
  Events, Interest, Poll, Token,
};
use rand::rngs::OsRng;
use rsa::RSAPrivateKey;
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc};

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

const JAVA_LISTENER: Token = Token(0xffffffff);
const BEDROCK_LISTENER: Token = Token(0xfffffffe);

pub struct Listener<'a> {
  java_listener: TcpListener,
  poll:          Poll,
  next_token:    usize,
  clients:       HashMap<Token, Conn<'a, JavaStream>>,
}

pub fn run(config: Config) -> Result<()> {
  let level = config.get("log-level");
  bb_common::init_with_level("proxy", level);

  let icon = Arc::new(load_icon(config.get("icon")));

  let addr = config.get::<&str>("address");
  info!("listening for java clients on {}", addr);
  let mut listener = Listener::new(addr)?;

  // let addr = "0.0.0.0:19132";
  // info!("listening for bedrock clients on {}", addr);
  // let mut bedrock_listener = bedrock::Listener::bind(addr).await?;

  // The vanilla server uses 1024 bits for this.
  let key = Arc::new(RSAPrivateKey::new(&mut OsRng, 1024).expect("failed to generate a key"));
  let der_key = if config.get("encryption") { Some(der::encode(&key)) } else { None };
  let server_ip: SocketAddr = config.get::<&str>("server").parse().unwrap();
  let compression = config.get("compression-thresh");

  let mut events = Events::with_capacity(1024);

  let conv = Arc::new(TypeConverter::new());

  loop {
    loop {
      match listener.poll.poll(&mut events, None) {
        Ok(()) => break,
        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
        Err(e) => return Err(e.into()),
      }
    }

    for event in &events {
      listener.handle(event, |client, server_token| {
        Conn::new(
          JavaStream::new(client),
          server_ip,
          key.clone(),
          der_key.clone(),
          server_token,
          conv.clone(),
        )
        .with_icon(&icon)
        .with_compression(compression)
      })?;
    }
  }
}

impl<'a> Listener<'a> {
  pub fn new(addr: &str) -> Result<Self> {
    let mut java_listener = TcpListener::bind(addr.parse()?)?;
    let poll = Poll::new()?;
    poll.registry().register(&mut java_listener, JAVA_LISTENER, Interest::READABLE)?;
    Ok(Listener { java_listener, poll, next_token: 0, clients: HashMap::new() })
  }
  pub fn handle(
    &mut self,
    event: &Event,
    new_client: impl Fn(TcpStream, Token) -> Conn<'a, JavaStream>,
  ) -> io::Result<()> {
    match event.token() {
      JAVA_LISTENER => {
        loop {
          match self.java_listener.accept() {
            Ok((mut client, _)) => {
              // This is the tcp stream connected to the client
              let client_token = Token(self.next_token);
              // This is the tcp stream connected to the server
              let server_token = Token(self.next_token + 1);
              self.next_token += 2;

              // Register this client for events
              self.poll.registry().register(
                &mut client,
                client_token,
                Interest::READABLE | Interest::WRITABLE,
              )?;
              // We will register the server tcp connection later, once we are done
              // handshaking.
              self.clients.insert(client_token, new_client(client, server_token));
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
            let conn = self.clients.get_mut(&token).expect("client doesn't exist!");
            match conn.read_server() {
              Ok(_) => {}
              Err(e) => self.handle_err(e, token),
            }
          }

          if event.is_writable() {
            if let Some(conn) = self.clients.get_mut(&token) {
              match conn.write_server() {
                Ok(_) => {}
                Err(e) => self.handle_err(e, token),
              }
            }
          }
        } else {
          if event.is_readable() {
            let conn = self.clients.get_mut(&token).expect("client doesn't exist!");
            match conn.read_client(self.poll.registry()) {
              Ok(false) => {}
              Ok(true) => {
                // This means the server wants to remove the client, and we have already logged
                // anything if needed, so we don' use `handle_and_should_close` here.
                self.clients.remove(&token);
              }
              Err(e) => self.handle_err(e, token),
            }
          }
          // The order here is important. If we are handshaking, then reading a packet
          // will probably prompt a direct response. In this arrangement, we can send more
          // packets before going back to poll().
          if event.is_writable() {
            if let Some(conn) = self.clients.get_mut(&token) {
              match conn.write_client() {
                Ok(_) => {}
                Err(e) => self.handle_err(e, token),
              }
            }
          }
        }
      }
    }
    Ok(())
  }

  /// Logs any errors that need to be logged, and removes the client if needed.
  fn handle_err(&mut self, e: Error, token: Token) {
    let remove = match e.io_kind() {
      Some(io::ErrorKind::WouldBlock) => false,
      Some(io::ErrorKind::ConnectionAborted) => {
        info!("client {:?} has disconnected", token);
        true
      }
      _ => {
        error!("error while flushing packets to the client {:?}: {}", token, e);
        true
      }
    };
    if remove {
      self.clients.remove(&token);
    }
  }
}

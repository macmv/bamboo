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

pub use conn::{JsonPlayer, JsonPlayers, JsonStatus, JsonVersion};
pub use error::{Error, Result};

use bb_common::{
  config::Config,
  math::der,
  util::chat::{Chat, Color},
  version::ProtocolVersion,
};
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

type ClientMap<'listener> = HashMap<Token, Conn<'listener, JavaStream>>;

pub struct Listener<'listener> {
  java_listener: TcpListener,
  poll:          Poll,
  next_token:    usize,
  clients:       ClientMap<'listener>,
}

struct TokenHandler<'listener, 'a> {
  token:   Token,
  clients: &'a mut ClientMap<'listener>,
}

/// Loads the config at the given path, using the server-provided default
/// config.
pub fn load_config(path: &str) -> Config { Config::new(path, include_str!("default.toml")) }
/// Loads the config at the given path, using the server-provided default
/// config. This will then write the default config to the `default` path
/// provided.
pub fn load_config_write_default(path: &str, default: &str) -> Config {
  Config::new_write_default(path, default, include_str!("default.toml"))
}

pub struct Proxy {
  icon:           Option<String>,
  key:            Arc<RSAPrivateKey>,
  der_key:        Option<Vec<u8>>,
  addr:           SocketAddr,
  server_addr:    SocketAddr,
  compression:    i32,
  conv:           Arc<TypeConverter>,
  status_builder: Arc<dyn for<'a> Fn(&'a str, ProtocolVersion) -> JsonStatus<'a>>,
}

impl Proxy {
  /// Creates a proxy with default settings.
  pub fn new(addr: SocketAddr, server_addr: SocketAddr) -> Self {
    Proxy {
      icon: None,
      key: Arc::new(RSAPrivateKey::new(&mut OsRng, 1024).expect("failed to generate a key")),
      der_key: None,
      addr,
      server_addr,
      compression: 256,
      conv: Arc::new(TypeConverter::new()),
      status_builder: Arc::new(|icon, ver| {
        let mut description = Chat::empty();
        description.add("Bamboo").color(Color::BrightGreen);
        description.add(" -- ").color(Color::Gray);
        #[cfg(debug_assertions)]
        description.add("Development mode").color(Color::Blue);
        #[cfg(not(debug_assertions))]
        description.add("Release mode").color(Color::Red);
        JsonStatus {
          version: JsonVersion {
            name:     format!("1.8 - {}", ProtocolVersion::latest()),
            protocol: if ver == ProtocolVersion::Invalid {
              ProtocolVersion::latest().id()
            } else {
              ver.id()
            } as i32,
          },
          players: JsonPlayers {
            max:    69,
            online: 420,
            sample: vec![JsonPlayer {
              name: "macmv".into(),
              id:   "a0ebbc8d-e0b0-4c23-a965-efba61ff0ae8".into(),
            }],
          },
          description,
          favicon: icon,
        }
      }),
    }
    .with_encryption(true)
  }
  /// Creates a proxy from the given config.
  pub fn from_config(config: Config) -> Result<Self> {
    Ok(
      Self::new(config.get::<&str>("address").parse()?, config.get::<&str>("server").parse()?)
        .with_encryption(config.get("encryption"))
        .with_compression(config.get("compression-thresh"))
        .with_icon(config.get("icon")),
    )
  }
  /// Enables or disables encryption for this connection.
  pub fn with_encryption(mut self, encryption: bool) -> Self {
    if encryption {
      self.der_key = Some(der::encode(&self.key));
    } else {
      self.der_key = None
    }
    self
  }
  /// Sets the compression threshold for the proxy. Set to `-1` to disable
  /// compression, and set to `0` to compress all packets.
  pub fn with_compression(mut self, compression: i32) -> Self {
    self.compression = compression;
    self
  }
  /// Sets the icon path for the proxy. This will be shown to all clients on
  /// the server list screen.
  pub fn with_icon(mut self, path: &str) -> Self {
    self.icon = Some(load_icon(path));
    self
  }

  /// Creates a new connection for the given stream.
  fn new_conn(&self, stream: JavaStream, server_token: Token) -> Conn<JavaStream> {
    let conn = Conn::new(
      stream,
      self.server_addr,
      self.key.clone(),
      self.der_key.clone(),
      server_token,
      self.conv.clone(),
      self.status_builder.clone(),
    )
    .with_compression(self.compression);
    if let Some(icon) = &self.icon {
      conn.with_icon(icon)
    } else {
      conn
    }
  }

  /// Runs the proxy with the given config. This will block until the proxy
  /// disconnects.
  pub fn run(&self) -> Result<()> {
    info!("listening for java clients on {}", self.addr);
    let mut listener = Listener::new(self.addr)?;

    // let addr = "0.0.0.0:19132";
    // info!("listening for bedrock clients on {}", addr);
    // let mut bedrock_listener = bedrock::Listener::bind(addr).await?;

    // The vanilla server uses 1024 bits for this.
    let mut events = Events::with_capacity(1024);

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
          self.new_conn(JavaStream::new(client), server_token)
        })?;
      }
    }
  }
}

impl<'a> Listener<'a> {
  pub fn new(addr: SocketAddr) -> Result<Self> {
    let mut java_listener = TcpListener::bind(addr)?;
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

        let mut handler = TokenHandler { token, clients: &mut self.clients };

        if is_server {
          if event.is_readable() {
            if let Some(conn) = handler.get() {
              let res = conn.read_server(self.poll.registry());
              handler.handle_bool(res);
            }
          }

          if event.is_writable() {
            if let Some(conn) = handler.get() {
              let res = conn.write_server();
              handler.handle_unit(res);
            }
          }
        } else {
          if event.is_readable() {
            if let Some(conn) = handler.get() {
              let res = conn.read_client(self.poll.registry());
              handler.handle_bool(res);
            }
          }
          // The order here is important. If we are handshaking, then reading a packet
          // will probably prompt a direct response. In this arrangement, we can send more
          // packets before going back to poll().
          if event.is_writable() {
            if let Some(conn) = handler.get() {
              let res = conn.write_client();
              handler.handle_unit(res);
            }
          }
        }
      }
    }
    Ok(())
  }
}

impl<'listener: 'b, 'b> TokenHandler<'listener, 'b> {
  pub fn get(&mut self) -> Option<&mut Conn<'listener, JavaStream>> {
    self.clients.get_mut(&self.token)
  }
  pub fn handle_unit(&mut self, res: Result<()>) {
    match res {
      Ok(()) => {}
      Err(e) => self.handle_err(e),
    }
  }
  pub fn handle_bool(&mut self, res: Result<bool>) {
    match res {
      Ok(false) => {}
      Ok(true) => {
        self.clients.remove(&self.token);
      }
      Err(e) => self.handle_err(e),
    }
  }

  /// Logs any errors that need to be logged, and removes the client if needed.
  fn handle_err(&mut self, e: Error) {
    let remove = match e.io_kind() {
      Some(io::ErrorKind::WouldBlock) => false,
      Some(io::ErrorKind::ConnectionAborted) => {
        info!("client {:?} has disconnected", self.token);
        true
      }
      _ => {
        error!("error while flushing packets to the client {:?}: {}", self.token, e);
        true
      }
    };
    if remove {
      self.clients.remove(&self.token);
    }
  }
}

use crate::stream::PacketStream;
use crossbeam_channel::{Sender, TryRecvError};
use mio::{net::TcpStream, Token, Waker};
use rand::{rngs::OsRng, RngCore};
use reqwest::StatusCode;
use rsa::{padding::PaddingScheme, RSAPrivateKey};
use sc_common::{
  math,
  net::{cb, sb, tcp},
  proto,
  util::{chat::Color, Chat, UUID},
  version::ProtocolVersion,
};
use sc_transfer::MessageWrite;
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{
  collections::VecDeque,
  convert::TryInto,
  fmt, io,
  io::{ErrorKind, Write},
  net::SocketAddr,
  sync::Arc,
};
use tonic::Streaming;

#[derive(Debug, Copy, Clone)]
pub enum State {
  Handshake,
  Status,
  Login,
  Play,
  Invalid,
}

impl State {
  fn from_next(next: i32) -> Self {
    if next == 1 {
      Self::Status
    } else if next == 2 {
      Self::Login
    } else {
      Self::Invalid
    }
  }
}

pub struct Conn<'a, S> {
  stream:       S,
  state:        State,
  ver:          ProtocolVersion,
  icon:         &'a str,
  /// The name sent from the client. The mojang auth server also sends us a
  /// username; we use this to validate the client info with the mojang auth
  /// info.
  username:     Option<String>,
  info:         Option<LoginInfo>,
  /// The four byte verify token, used by the client in encryption.
  verify_token: [u8; 4],

  /// The private key. Always present, even if encryption is disabled.
  key:                Arc<RSAPrivateKey>,
  /// The der encoded public key. None if we don't want encryption.
  der_key:            Option<Vec<u8>>,
  /// Used in handshake. This is different from `stream.compression`
  compression_target: i32,

  /// Set when the connection is closed.
  closed: bool,

  /// Server address. Used when we are done handshaking, and need to connect to
  /// a server.
  addr:         SocketAddr,
  /// Used when we make a server listener.
  client_token: Token,
  /// Used when we create the tcp stream connected to the server.
  server_token: Token,
  /// A connection to the server. If none, then we haven't finished handshaking.
  server:       Option<TcpStream>,
  /// Bytes that need to be written to the server. Will be empty if we have
  /// written everything.
  serverbound:  Vec<u8>,
  /// Used to encode packets. The data in here is undefined, but the length will
  /// be constant.
  garbage:      Vec<u8>,
}

impl<S: fmt::Debug> fmt::Debug for Conn<'_, S> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("JavaStream")
      .field("stream", &self.stream)
      .field("state", &self.state)
      .field("ver", &self.ver)
      .field("username", &self.username)
      .field("closed", &self.closed)
      .finish()
  }
}

pub struct ServerListener {
  client: crossbeam_channel::Sender<tcp::Packet>,
  server: Streaming<proto::Packet>,
  ver:    ProtocolVersion,
  token:  Token,
}

#[derive(Deserialize, Debug)]
pub struct LoginInfo {
  pub id:         UUID,
  // Player's username
  pub name:       String,
  // Things like textures
  pub properties: Vec<LoginProperty>,
}

#[derive(Deserialize, Debug)]
pub struct LoginProperty {
  // Example: "textures"
  pub name:      String,
  // Example: base64 encoded png
  pub value:     String,
  // Example: base64 signature, signed with Yggdrasil's private key
  pub signature: Option<String>,
}

impl ServerListener {
  /// This starts listening for packets from the server. The rx and tx are used
  /// to close the ClientListener. Specifically, the tx will send a value once
  /// this listener has been closed, and this listener will close once the rx
  /// gets a message.
  pub async fn run(&mut self, waker: Arc<Waker>, needs_flush_tx: Sender<Token>) -> io::Result<()> {
    let res = self.run_inner(waker, needs_flush_tx).await;
    // Close connection here
    res
  }
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }
  async fn run_inner(
    &mut self,
    waker: Arc<Waker>,
    needs_flush_tx: Sender<Token>,
  ) -> io::Result<()> {
    // loop {
    //   match self
    //     .server
    //     .message()
    //     .await
    //     .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?
    //   {
    //     Some(p) => {
    //       self.client.send(cb::Packet::from_proto(p,
    // self.ver).to_tcp(self.ver)).unwrap();       needs_flush_tx.send(self.
    // token).unwrap();       waker.wake()?;
    //     }
    //     None => break,
    //   }
    // }
    Ok(())
  }
}

#[derive(Serialize)]
struct JsonStatus<'a> {
  version:     JsonVersion,
  players:     JsonPlayers,
  description: Chat,
  favicon:     &'a str,
}

#[derive(Serialize)]
struct JsonVersion {
  name:     String,
  protocol: i32,
}

#[derive(Serialize)]
struct JsonPlayers {
  max:    i32,
  online: i32,
  sample: Vec<JsonPlayer>,
}

#[derive(Serialize)]
struct JsonPlayer {
  name: String,
  id:   String,
}

impl<'a, S: PacketStream + Send + Sync> Conn<'a, S> {
  pub fn new(
    stream: S,
    addr: SocketAddr,
    compression_target: i32,
    key: Arc<RSAPrivateKey>,
    der_key: Option<Vec<u8>>,
    icon: &'a str,
    client_token: Token,
    server_token: Token,
  ) -> Result<Conn<'a, S>, tonic::Status> {
    Ok(Conn {
      stream,
      state: State::Handshake,
      ver: ProtocolVersion::Invalid,
      icon,
      username: None,
      info: None,
      verify_token: [0u8; 4],
      key,
      der_key,
      compression_target,
      closed: false,
      addr,
      client_token,
      server_token,
      server: None,
      serverbound: Vec::with_capacity(16 * 1024),
      garbage: vec![0; 64 * 1024],
    })
  }
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }
  pub fn closed(&self) -> bool {
    self.closed
  }

  async fn connect_to_server(&mut self) -> Result<(), io::Error> {
    self.server = Some(TcpStream::connect(self.addr)?);

    let mut buf = [0; 1024];
    let mut m = MessageWrite::new(&mut buf);
    m.write_str(self.username.as_ref().unwrap()).unwrap();
    m.write_bytes(&self.info.as_ref().unwrap().id.as_be_bytes()).unwrap();
    m.write_i32(self.ver.id() as i32).unwrap();
    let idx = m.index();
    self.server.as_mut().unwrap().write(&buf[..idx])?;

    Ok(())
  }

  /// Writes all the data possible to the client. Returns Err(WouldBlock) or
  /// Ok(()) if everything worked as expected.
  pub fn write_client(&mut self) -> io::Result<()> {
    while self.stream.needs_flush() {
      self.stream.flush()?;
    }
    Ok(())
  }

  /// Reads as much data as possible, without blocking. Returns Ok(true) or
  /// Err(_) if the connection should be closed.
  pub fn read_client(&mut self) -> io::Result<bool> {
    // Poll, then read, then try and poll again. If we have no data left, we return
    // Ok(false). If we have closed the connection, we return Ok(true). If we error
    // at all, we close the connection.
    loop {
      match self.poll() {
        Ok(_) => match self.read() {
          Ok(_) => {
            if self.closed() {
              return Ok(true);
            }
          }
          Err(e) => return Err(e),
        },
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(false),
        Err(e) => return Err(e),
      }
    }
  }
  fn poll(&mut self) -> io::Result<()> {
    self.stream.poll()
  }
  /// Reads a packet from the internal buffer from the client. Does not interact
  /// with the tcp connection at all.
  fn read(&mut self) -> io::Result<()> {
    loop {
      match self.stream.read(self.ver) {
        Ok(Some(p)) => match self.state {
          State::Play => {
            let packet = sb::Packet::from_tcp(p, self.ver);
            self.send_to_server(packet);
          }
          _ => {
            futures::executor::block_on(self.handle_handshake(p))?;
          }
        },
        Ok(None) => break,
        Err(e) => return Err(e),
      }
    }
    Ok(())
  }

  /// Tries to send the packet to the server, and buffers it if that is not
  /// possible.
  fn send_to_server(&mut self, p: sb::Packet) -> io::Result<()> {
    // The only error here is EOF, which means 64k was not enough space for this
    // packet.
    let n = p.to_sc(self.ver, &mut self.garbage).unwrap();
    let buf = &self.garbage[..n];
    self.serverbound.extend_from_slice(buf);
    self.server_flush()
  }

  fn server_flush(&mut self) -> io::Result<()> {
    let n = self.server.as_mut().unwrap().write(&self.serverbound)?;
    self.serverbound.drain(0..n);
    Ok(())
  }

  /// Sends the set compression packet, using self.compression_target. The
  /// stream will not be flushed.
  fn send_compression(&mut self) {
    // Set compression, only if the thresh hold is non-zero
    if self.compression_target != 0 {
      let mut out = tcp::Packet::new(3, self.ver());
      out.write_varint(self.compression_target);
      self.stream.write(out);
      // Must happen after the packet has been sent
      self.stream.set_compression(self.compression_target);
    }
  }

  /// Sends the login success packet, and sets the state to Play. The stream
  /// will not be flushed.
  async fn finish_login(&mut self) -> Result<(), io::Error> {
    // Login success
    let info = self.info.as_ref().unwrap();
    let ver = self.ver();
    let mut out = tcp::Packet::new(2, ver);
    if ver >= ProtocolVersion::V1_16 {
      out.write_uuid(info.id);
    } else {
      out.write_str(&info.id.as_dashed_str());
    }
    out.write_str(&info.name);
    self.stream.write(out);

    self.state = State::Play;
    self.connect_to_server().await?;

    Ok(())
  }

  // Disconnects the client during authentication. The stream will not be flushed.
  fn send_disconnect<C: Into<Chat>>(&mut self, reason: C) {
    // Disconnect
    let mut out = tcp::Packet::new(0, self.ver);
    out.write_str(&reason.into().to_json());
    self.stream.write(out);
  }

  /// Generates the json status for the server
  fn build_status(&self) -> JsonStatus {
    let mut description = Chat::empty();
    description.add("Sugarcane").color(Color::BrightGreen);
    description.add(" -- ").color(Color::Gray);
    #[cfg(debug_assertions)]
    description.add("Development mode").color(Color::Blue);
    #[cfg(not(debug_assertions))]
    description.add("Release mode").color(Color::Red);
    JsonStatus {
      version: JsonVersion { name: "1.8".into(), protocol: self.ver.id() as i32 },
      players: JsonPlayers {
        max:    69,
        online: 420,
        sample: vec![JsonPlayer {
          name: "macmv".into(),
          id:   "a0ebbc8d-e0b0-4c23-a965-efba61ff0ae8".into(),
        }],
      },
      description,
      favicon: self.icon,
    }
  }

  /// Runs the entire login process with the client. If compression is 0, then
  /// compression will not be enabled. The key should always be the server's
  /// private key, even if it won't be used. The der_key should be the der
  /// encoded public key. Although this function could just generate that from
  /// the private key, this is faster. Also, if the der_key is None, then
  /// encryption will be disabled.
  ///
  /// This function will return an error if anything goes wrong. The client
  /// should always be kicked if an error is returned. If Ok(None) is returned,
  /// then the client will have already closed the connection. This is for when
  /// the client just wants to get the status of the server. If this function
  /// returns Ok(Some(LoginInfo)), then a connection should be initialized with
  /// a grpc server.
  async fn handle_handshake(&mut self, mut p: tcp::Packet) -> io::Result<()> {
    match self.state {
      State::Handshake => {
        if p.id() != 0 {
          return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("unknown handshake packet {}", p.id()),
          ));
        }
        self.ver = ProtocolVersion::from(p.read_varint());
        if self.ver == ProtocolVersion::Invalid {
          return Err(io::Error::new(ErrorKind::InvalidInput, "client sent an invalid version"));
        }

        let _addr = p.read_str();
        let _port = p.read_u16();
        let next = p.read_varint();
        self.state = State::from_next(next);
      }
      State::Status => {
        match p.id() {
          // Server status
          0 => {
            let status = self.build_status();
            let mut out = tcp::Packet::new(0, self.ver);
            out.write_str(&serde_json::to_string(&status).unwrap());
            self.stream.write(out);
          }
          // Ping
          1 => {
            let id = p.read_u64();
            // Send pong
            let mut out = tcp::Packet::new(1, self.ver);
            out.write_u64(id);
            self.stream.write(out);
            self.stream.flush()?;
            // Client is done sending packets, we can close now.
            self.closed = true;
            return Ok(());
          }
          _ => {
            return Err(io::Error::new(
              ErrorKind::InvalidInput,
              format!("unknown status packet {}", p.id()),
            ));
          }
        }
      }
      State::Login => {
        match p.id() {
          // Login start
          0 => {
            if self.username.is_some() {
              return Err(io::Error::new(ErrorKind::InvalidInput, "client sent two login packets"));
            }
            let name = p.read_str();
            self.username = Some(name.to_string());
            if self.der_key.is_none() {
              self.info = Some(LoginInfo {
                // Generate uuid if we are in offline mode
                id: UUID::from_bytes(*md5::compute(&name)),
                name,
                properties: vec![],
              });
            }

            match &self.der_key {
              Some(key) => {
                // Make sure to actually generate a token
                OsRng.fill_bytes(&mut self.verify_token);

                // Encryption request
                let mut out = tcp::Packet::new(1, self.ver);
                out.write_str(""); // Server id, should be empty
                out.write_varint(key.len() as i32); // Key len
                out.write_buf(key); // DER encoded RSA key
                out.write_varint(4); // Token len
                out.write_buf(&self.verify_token); // Verify token
                self.stream.write(out);
                // Wait for encryption response to enable encryption
              }
              None => {
                self.send_compression();
                self.finish_login().await?;
              }
            }
          }
          // Encryption response
          1 => {
            if self.username.is_none() {
              return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "client did not send login start before sending ecryption response",
              ));
            }
            let len = p.read_varint();
            let recieved_secret = p.read_buf(len);
            let len = p.read_varint();
            let recieved_token = p.read_buf(len);

            let decrypted_secret =
              self.key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recieved_secret).map_err(|e| {
                io::Error::new(ErrorKind::InvalidInput, format!("unable to decrypt secret: {}", e))
              })?;
            let decrypted_token =
              self.key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recieved_token).map_err(|e| {
                io::Error::new(ErrorKind::InvalidInput, format!("unable to decrypt token: {}", e))
              })?;

            // Make sure the client sent the correct verify token back
            if decrypted_token != self.verify_token {
              return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                  "invalid verify token recieved from client (len: {})",
                  decrypted_token.len()
                ),
              ));
            }
            let len = decrypted_secret.len();
            let secret = match decrypted_secret.try_into() {
              Ok(v) => v,
              Err(_) => {
                return Err(io::Error::new(
                  ErrorKind::InvalidInput,
                  format!("invalid secret recieved from client (len: {}, expected len 16)", len,),
                ))
              }
            };

            // If we need to disconnect the client, the client is expecting encrypted
            // packets from now on, so we need to enable encryption here.
            self.stream.enable_encryption(&secret);
            self.stream.enable_encryption(&secret);

            let mut hash = Sha1::new();
            hash.update("");
            hash.update(secret);
            hash.update(self.der_key.as_ref().unwrap());
            self.info = match reqwest::blocking::get(format!(
              "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
              self.username.as_ref().unwrap(),
              math::hexdigest(hash)
            )) {
              Ok(v) => {
                info!("got status code: {}", v.status());
                if v.status() == StatusCode::NO_CONTENT {
                  self.send_disconnect("Invalid auth token! Please re-login (restart your game and launcher)");
                  self.stream.flush()?;
                  // Disconnect client; they are not authenticated
                  self.closed = true;
                  return Ok(());
                }
                match v.json() {
                  Ok(v) => Some(v),
                  Err(e) => return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid json data recieved from session server: {}", e),
                  ))
                }
              },
              Err(e) => return Err(io::Error::new(
                ErrorKind::Other,
                format!("failed to authenticate client: {}", e),
              ))
            };

            self.send_compression();
            self.finish_login().await?;
          }
          _ => {
            return Err(io::Error::new(
              ErrorKind::InvalidInput,
              format!("unknown login packet {}", p.id()),
            ));
          }
        }
      }
      v => {
        return Err(io::Error::new(
          ErrorKind::InvalidInput,
          format!("invalid connection state {:?}", v),
        ));
      }
    }

    self.stream.flush()?;
    Ok(())
  }
}

use crate::stream::PacketStream;
use mio::{net::TcpStream, Interest, Registry, Token};
use rand::{rngs::OsRng, RngCore};
use rsa::{padding::PaddingScheme, RSAPrivateKey};
use sc_common::{
  math,
  net::{cb, sb, tcp},
  util::{chat::Color, Chat, UUID},
  version::ProtocolVersion,
};
use sc_transfer::{MessageReader, MessageWriter, ReadError};
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{
  convert::TryInto,
  fmt, io,
  io::{ErrorKind, Read, Write},
  net::SocketAddr,
  sync::Arc,
};

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
  client_stream: S,
  state:         State,
  ver:           ProtocolVersion,
  icon:          &'a str,
  /// The name sent from the client. The mojang auth server also sends us a
  /// username; we use this to validate the client info with the mojang auth
  /// info.
  username:      Option<String>,
  info:          Option<LoginInfo>,
  /// The four byte verify token, used by the client in encryption.
  verify_token:  [u8; 4],

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
  addr:          SocketAddr,
  /// Used when we create the tcp stream connected to the server.
  server_token:  Token,
  /// A connection to the server. If none, then we haven't finished handshaking.
  server_stream: Option<TcpStream>,
  /// Bytes that need to be written to the server. Will be empty if we have
  /// written everything.
  to_server:     Vec<u8>,
  /// Bytes that have been read from the server, but don't form a complete
  /// packet yet.
  from_server:   Vec<u8>,
  /// Used to encode packets. The data in here is undefined, but the length will
  /// be constant.
  garbage:       Vec<u8>,
}

impl<S: fmt::Debug> fmt::Debug for Conn<'_, S> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("JavaStream")
      .field("client_stream", &self.client_stream)
      .field("state", &self.state)
      .field("ver", &self.ver)
      .field("username", &self.username)
      .field("closed", &self.closed)
      .finish()
  }
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
    client_stream: S,
    addr: SocketAddr,
    compression_target: i32,
    key: Arc<RSAPrivateKey>,
    der_key: Option<Vec<u8>>,
    icon: &'a str,
    server_token: Token,
  ) -> Conn<'a, S> {
    Conn {
      client_stream,
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
      server_stream: None,
      server_token,
      to_server: Vec::with_capacity(16 * 1024),
      from_server: Vec::with_capacity(16 * 1024),
      garbage: vec![0; 64 * 1024],
    }
  }
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }
  pub fn closed(&self) -> bool {
    self.closed
  }

  fn connect_to_server(&mut self, reg: &Registry) -> Result<(), io::Error> {
    info!("connecting to server at {:?}", self.addr);
    let mut stream = TcpStream::connect(self.addr)?;
    reg.register(&mut stream, self.server_token, Interest::READABLE | Interest::WRITABLE).unwrap();
    self.server_stream = Some(stream);

    let mut buf = [0; 1024];
    let mut m = MessageWriter::new(&mut buf);
    m.write_str(self.username.as_ref().unwrap()).unwrap();
    m.write_bytes(&self.info.as_ref().unwrap().id.as_be_bytes()).unwrap();
    m.write_i32(self.ver.id() as i32).unwrap();
    let len = m.index();
    let mut prefix = [0; 5];
    let mut m = MessageWriter::new(&mut prefix);
    m.write_i32(len as i32).unwrap();
    let prefix_len = m.index();
    self.to_server.extend_from_slice(&prefix[..prefix_len]);
    self.to_server.extend_from_slice(&buf[..len]);
    self.write_server()
  }

  pub fn write_server(&mut self) -> io::Result<()> {
    let n = self.server_stream.as_mut().unwrap().write(&self.to_server)?;
    self.to_server.drain(0..n);
    Ok(())
  }

  /// Reads as much data as possible from the server. Returns Ok(true) or Err(_)
  /// if the connection should be terminated.
  pub fn read_server(&mut self) -> io::Result<bool> {
    loop {
      match self.poll_server() {
        Ok(true) => return Ok(true),
        Ok(false) => loop {
          match self.read_server_packet() {
            Ok(true) => {}
            Ok(false) => break,
            Err(e) => return Err(e),
          };
          if self.closed() {
            return Ok(true);
          }
        },
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(false),
        Err(e) => return Err(e),
      }
    }
  }

  /// Returns true if the connection should close.
  fn poll_server(&mut self) -> io::Result<bool> {
    let n = self.server_stream.as_mut().unwrap().read(&mut self.garbage)?;
    if n == 0 {
      return Ok(true);
    }
    self.from_server.extend_from_slice(&self.garbage[..n]);
    Ok(false)
  }

  /// Returns true if there are more packets, false if there is an empty or
  /// partial packet.
  fn read_server_packet(&mut self) -> io::Result<bool> {
    let mut m = MessageReader::new(&self.from_server);
    match m.read_i32() {
      Ok(len) => {
        if len as usize + m.index() <= self.from_server.len() {
          let idx = m.index();
          self.from_server.drain(0..idx);
          let mut m = MessageReader::new(&self.from_server[..len as usize]);
          let p = cb::Packet::from_sc(&mut m, self.ver).map_err(|err| {
            io::Error::new(
              io::ErrorKind::InvalidData,
              format!("while reading field packet got error: {}", err),
            )
          })?;
          let parsed = m.index();
          if len as usize != parsed {
            return Err(io::Error::new(
              io::ErrorKind::InvalidData,
              format!(
                "did not read all the packet data (expected to read {} bytes, but only read {} bytes)",
                len, parsed
              ),
            ));
          }
          self.from_server.drain(0..parsed);
          self.send_to_client(p)?;
          Ok(true)
        } else {
          Ok(false)
        }
      }
      // There are no bytes waiting for us, or an invalid varint.
      Err(e) => {
        if matches!(e, ReadError::EOF) {
          Ok(false)
        } else {
          return Err(io::Error::new(ErrorKind::InvalidData, e.to_string()));
        }
      }
    }
  }

  /// Writes all the data possible to the client. Returns Err(WouldBlock) or
  /// Ok(()) if everything worked as expected.
  pub fn write_client(&mut self) -> io::Result<()> {
    while self.client_stream.needs_flush() {
      self.client_stream.flush()?;
    }
    Ok(())
  }

  /// Reads as much data as possible, without blocking. Returns Ok(true) or
  /// Err(_) if the connection should be closed.
  pub fn read_client(&mut self, reg: &Registry) -> io::Result<bool> {
    // Poll, then read, then try and poll again. If we have no data left, we return
    // Ok(false). If we have closed the connection, we return Ok(true). If we error
    // at all, we close the connection.
    loop {
      match self.poll_client() {
        Ok(_) => match self.read_client_packet(reg) {
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
  /// Reads data from the client tcp connection, and buffers that to be read in
  /// `read_client_packet`.
  fn poll_client(&mut self) -> io::Result<()> {
    self.client_stream.poll()
  }
  /// Reads a packet from the internal buffer from the client. Does not interact
  /// with the tcp connection at all.
  fn read_client_packet(&mut self, reg: &Registry) -> io::Result<()> {
    loop {
      match self.client_stream.read(self.ver) {
        Ok(Some(mut p)) => match self.state {
          State::Play => {
            let packet = sb::Packet::from_tcp(&mut p, self.ver);
            self.send_to_server(packet)?;
          }
          _ => {
            self.handle_handshake(p, reg)?;
          }
        },
        Ok(None) => break,
        Err(e) => return Err(e),
      }
    }
    Ok(())
  }

  /// Tries to send the packet to the client, and buffers it if that is not
  /// possible.
  fn send_to_client(&mut self, p: cb::Packet) -> io::Result<()> {
    let mut tcp = tcp::Packet::new(p.tcp_id(self.ver).try_into().unwrap(), self.ver);
    p.to_tcp(&mut tcp);
    self.client_stream.write(tcp);
    self.write_client()
  }

  /// Tries to send the packet to the server, and buffers it if that is not
  /// possible.
  fn send_to_server(&mut self, p: sb::Packet) -> io::Result<()> {
    // The only error here is EOF, which means the garbage buffer was not enough
    // space for this packet.

    // TODO: Repalce tcp::Packet with MessageWrite
    let mut m = MessageWriter::new(&mut self.garbage);
    p.to_sc(&mut m);

    let mut prefix = [0; 5];
    let mut m = MessageWriter::new(&mut prefix);
    m.write_u32(m.index() as u32).unwrap();
    let prefix_len = m.index();
    self.to_server.extend_from_slice(&prefix[..prefix_len]);
    self.to_server.extend_from_slice(&self.garbage);

    self.write_server()
  }

  /// Sends the set compression packet, using self.compression_target. The
  /// stream will not be flushed.
  fn send_compression(&mut self) {
    // Set compression, only if the thresh hold is non-zero
    if self.compression_target != 0 {
      let mut out = tcp::Packet::new(3, self.ver());
      out.write_varint(self.compression_target);
      self.client_stream.write(out);
      // Must happen after the packet has been sent
      self.client_stream.set_compression(self.compression_target);
    }
  }

  /// Sends the login success packet, and sets the state to Play. The stream
  /// will not be flushed.
  fn finish_login(&mut self, reg: &Registry) -> Result<(), io::Error> {
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
    self.client_stream.write(out);

    self.state = State::Play;
    self.connect_to_server(reg)?;

    Ok(())
  }

  // Disconnects the client during authentication. The stream will not be flushed.
  fn send_disconnect<C: Into<Chat>>(&mut self, reason: C) {
    // Disconnect
    let mut out = tcp::Packet::new(0, self.ver);
    out.write_str(&reason.into().to_json());
    self.client_stream.write(out);
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
  fn handle_handshake(&mut self, mut p: tcp::Packet, reg: &Registry) -> io::Result<()> {
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

        // Max len according to 1.17.1
        let _addr = p.read_str(255);
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
            self.client_stream.write(out);
          }
          // Ping
          1 => {
            let id = p.read_u64();
            // Send pong
            let mut out = tcp::Packet::new(1, self.ver);
            out.write_u64(id);
            self.client_stream.write(out);
            self.client_stream.flush()?;
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
            // Max length according to 1.17.1
            let name = p.read_str(16);
            self.username = Some(name.to_string());
            if self.der_key.is_none() {
              self.info = Some(LoginInfo {
                // Generate uuid if we are in offline mode
                id: UUID::from_be_bytes(*md5::compute(&name)),
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
                self.client_stream.write(out);
                // Wait for encryption response to enable encryption
              }
              None => {
                self.send_compression();
                self.finish_login(reg)?;
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
            let recieved_secret = p.read_buf(len.try_into().unwrap());
            let len = p.read_varint();
            let recieved_token = p.read_buf(len.try_into().unwrap());

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
            self.client_stream.enable_encryption(&secret);
            self.client_stream.enable_encryption(&secret);

            let mut hash = Sha1::new();
            hash.update("");
            hash.update(secret);
            hash.update(self.der_key.as_ref().unwrap());
            self.info = match ureq::get(&format!(
              "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
              self.username.as_ref().unwrap(),
              math::hexdigest(hash)
            )).call() {
              Ok(v) => {
                info!("got status code: {}", v.status());
                // No content
                if v.status() == 204 {
                  self.send_disconnect("Invalid auth token! Please re-login (restart your game and launcher)");
                  self.client_stream.flush()?;
                  // Disconnect client; they are not authenticated
                  self.closed = true;
                  return Ok(());
                }
                match serde_json::from_reader(v.into_reader()) {
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
            self.finish_login(reg)?;
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

    self.client_stream.flush()?;
    Ok(())
  }
}

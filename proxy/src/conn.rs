use crate::version::Generator;

use common::{
  math,
  net::{cb, sb, tcp},
  proto,
  proto::minecraft_client::MinecraftClient,
  stream::{StreamReader, StreamWriter},
  util::{chat::Color, Chat, UUID},
  version::ProtocolVersion,
};
use rand::{rngs::OsRng, RngCore};
use reqwest::StatusCode;
use rsa::{padding::PaddingScheme, RSAPrivateKey};
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{convert::TryInto, error::Error, io, io::ErrorKind, sync::Arc};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::channel::Channel, Request, Status, Streaming};

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

pub struct Conn<R, W> {
  reader: R,
  writer: W,
  server: MinecraftClient<Channel>,
  state:  State,
  gen:    Arc<Generator>,
  ver:    ProtocolVersion,
}

pub struct ClientListener<R> {
  client: R,
  server: mpsc::Sender<proto::Packet>,
  gen:    Arc<Generator>,
  ver:    ProtocolVersion,
}

pub struct ServerListener<W> {
  client: W,
  server: Streaming<proto::Packet>,
  gen:    Arc<Generator>,
  ver:    ProtocolVersion,
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

impl<R: StreamReader + Send> ClientListener<R> {
  /// This starts listening for packets from the server. The rx and tx are used
  /// to close the ServerListener. Specifically, the tx will send a value once
  /// this listener has been closed, and this listener will close once the rx
  /// gets a message.
  pub async fn run(
    &mut self,
    tx: oneshot::Sender<()>,
    rx: oneshot::Receiver<()>,
  ) -> Result<(), Box<dyn Error>> {
    let res = self.run_inner(rx).await;
    // Close the other connection. We ignore the result, as that means the rx has
    // been dropped. We don't care if the rx has been dropped, because that means
    // the other listener has already closed.
    let _ = tx.send(());
    res
  }
  async fn run_inner(&mut self, mut rx: oneshot::Receiver<()>) -> Result<(), Box<dyn Error>> {
    loop {
      tokio::select! {
        v = self.client.poll() => v?,
        _ = &mut rx => break,
      }
      loop {
        let p = self.client.read(self.ver).unwrap();
        let p = if let Some(v) = p { v } else { break };
        // Make sure there were no errors set within the packet during parsing
        match p.err() {
          Some(e) => {
            error!("error while parsing packet: {}", e);
            return Err(Box::new(io::Error::new(
              ErrorKind::InvalidData,
              "failed to parse packet, closing connection",
            )));
          }
          None => {}
        }
        // Converting a tcp packet to a grpc packet should always work. If it fails,
        // then it is either an invalid version, an unknown packet, or a parsing error.
        // If it is a parsing error, we want to close the connection.
        let sb = match self.gen.serverbound(self.ver, p) {
          Err(e) => match e.kind() {
            ErrorKind::Other => {
              return Err(Box::new(e));
            }
            _ => {
              warn!("{}", e);
              continue;
            }
          },
          Ok(v) => v,
        };
        trace!("sending proto: {:?}", &sb);
        self.server.send(sb.to_proto()).await?;
      }
    }
    Ok(())
  }

  /// Sends a packet to the server. Should only be used for things like login
  /// packets.
  pub async fn send_to_server(&mut self, p: sb::Packet) -> Result<(), Box<dyn Error>> {
    self.server.send(p.to_proto()).await?;
    Ok(())
  }
}

impl<W: StreamWriter + Send> ServerListener<W> {
  /// This starts listening for packets from the server. The rx and tx are used
  /// to close the ClientListener. Specifically, the tx will send a value once
  /// this listener has been closed, and this listener will close once the rx
  /// gets a message.
  pub async fn run(
    &mut self,
    tx: oneshot::Sender<()>,
    rx: oneshot::Receiver<()>,
  ) -> Result<(), Box<dyn Error>> {
    let res = self.run_inner(rx).await;
    let _ = tx.send(());
    res
  }
  async fn run_inner(&mut self, mut rx: oneshot::Receiver<()>) -> Result<(), Box<dyn Error>> {
    loop {
      let pb = self.server.message();
      let p;

      tokio::select! {
        v = pb => p = v?,
        _ = &mut rx => break,
      }
      let p = p.unwrap();
      let cb = self.gen.clientbound(self.ver, cb::Packet::from_proto(p))?;
      for p in cb {
        self.client.write(p).await?;
      }
    }
    Ok(())
  }
}

#[derive(Serialize)]
struct JsonStatus {
  version:     JsonVersion,
  players:     JsonPlayers,
  description: Chat,
  favicon:     String,
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

impl<R: StreamReader + Send, W: StreamWriter + Send> Conn<R, W> {
  pub async fn new(
    gen: Arc<Generator>,
    reader: R,
    writer: W,
    ip: String,
  ) -> Result<Self, tonic::transport::Error> {
    Ok(Conn {
      reader,
      writer,
      server: MinecraftClient::connect(ip).await?,
      state: State::Handshake,
      gen,
      ver: ProtocolVersion::Invalid,
    })
  }
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }

  pub async fn split(mut self) -> Result<(ClientListener<R>, ServerListener<W>), Status> {
    let (tx, rx) = mpsc::channel(1);

    let response = self.server.connection(Request::new(ReceiverStream::new(rx))).await?;
    let inbound = response.into_inner();

    Ok((
      ClientListener {
        ver:    self.ver,
        gen:    self.gen.clone(),
        client: self.reader,
        server: tx,
      },
      ServerListener { ver: self.ver, gen: self.gen, client: self.writer, server: inbound },
    ))
  }

  /// Sends the set compression packet, if needed.
  async fn send_compression(&mut self, compression: i32) -> io::Result<()> {
    // Set compression, only if the thresh hold is non-zero
    if compression != 0 {
      let mut out = tcp::Packet::new(3, self.ver);
      out.write_varint(compression);
      self.writer.write(out).await?;
      // Must happen after the packet has been sent
      self.writer.set_compression(compression);
      self.reader.set_compression(compression);
    }
    Ok(())
  }

  /// Sends the login success packet, and sets the state to Play.
  async fn send_success(&mut self, info: &LoginInfo) -> io::Result<()> {
    // Login success
    let mut out = tcp::Packet::new(2, self.ver);
    if self.ver >= ProtocolVersion::V1_16 {
      out.write_uuid(info.id);
    } else {
      out.write_str(&info.id.as_dashed_str());
    }
    out.write_str(&info.name);
    self.writer.write(out).await?;

    self.state = State::Play;
    Ok(())
  }

  // Disconnects the client during authentication
  async fn send_disconnect<C: Into<Chat>>(&mut self, reason: C) -> io::Result<()> {
    // Disconnect
    let mut out = tcp::Packet::new(0, self.ver);
    out.write_str(&reason.into().to_json());
    self.writer.write(out).await?;
    Ok(())
  }

  /// Generates the json status for the server
  fn build_status(&self) -> JsonStatus {
    let mut description = Chat::empty();
    description.add("Sugarcane").color(Color::BrightGreen);
    description.add(" -- ").color(Color::Gray);
    description.add("Development mode").color(Color::Blue);
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
      favicon: "".into(),
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
  pub async fn handshake(
    &mut self,
    compression: i32,
    key: RSAPrivateKey,
    der_key: Option<Vec<u8>>,
  ) -> io::Result<Option<LoginInfo>> {
    // The name sent from the client. The mojang auth server also sends us a
    // username; we use this to validate the client info with the mojang auth info.
    let mut username: Option<String> = None;
    let mut info = None;
    // The four byte verify token, used by the client in encryption.
    let mut token = [0u8; 4];
    'login: loop {
      self.reader.poll().await.unwrap();
      loop {
        let p = self.reader.read(self.ver).unwrap();
        if p.is_none() {
          break;
        }
        let mut p = p.unwrap();
        let err = p.err();
        match err {
          Some(e) => {
            error!("error while parsing packet: {}", e);
            break;
          }
          None => {}
        }
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
              return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "client sent an invalid version",
              ));
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
                self.writer.write(out).await?;
              }
              // Ping
              1 => {
                let id = p.read_u64();
                // Send pong
                let mut out = tcp::Packet::new(1, self.ver);
                out.write_u64(id);
                self.writer.write(out).await?;
                // Client is done sending packets, we can close now.
                return Ok(None);
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
                if username.is_some() {
                  return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "client sent two login packets",
                  ));
                }
                let name = p.read_str();
                username = Some(name.to_string());
                if der_key.is_none() {
                  info = Some(LoginInfo {
                    // Generate uuid if we are in offline mode
                    id: UUID::from_bytes(*md5::compute(&name)),
                    name,
                    properties: vec![],
                  });
                }

                match &der_key {
                  Some(key) => {
                    // Make sure to actually generate a token
                    OsRng.fill_bytes(&mut token);

                    // Encryption request
                    let mut out = tcp::Packet::new(1, self.ver);
                    out.write_str(""); // Server id, should be empty
                    out.write_varint(key.len() as i32); // Key len
                    out.write_buf(key); // DER encoded RSA key
                    out.write_varint(4); // Token len
                    out.write_buf(&token); // Verify token
                    self.writer.write(out).await?;
                    // Wait for encryption response to enable encryption
                  }
                  None => {
                    self.send_compression(compression).await?;
                    self.send_success(info.as_ref().unwrap()).await?;
                    // Successful login, we can break now
                    break 'login;
                  }
                }
              }
              // Encryption response
              1 => {
                if username.is_none() {
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
                  key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recieved_secret).map_err(|e| {
                    io::Error::new(
                      ErrorKind::InvalidInput,
                      format!("unable to decrypt secret: {}", e),
                    )
                  })?;
                let decrypted_token =
                  key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recieved_token).map_err(|e| {
                    io::Error::new(
                      ErrorKind::InvalidInput,
                      format!("unable to decrypt token: {}", e),
                    )
                  })?;

                // Make sure the client sent the correct verify token back
                if decrypted_token != token {
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
                      format!(
                        "invalid secret recieved from client (len: {}, expected len 16)",
                        len,
                      ),
                    ))
                  }
                };

                // If we need to disconnect the client, the client is expecting encrypted
                // packets from now on, so we need to enable encryption here.
                self.writer.enable_encryption(&secret);
                self.reader.enable_encryption(&secret);

                let mut hash = Sha1::new();
                hash.update("");
                hash.update(secret);
                hash.update(der_key.unwrap());
                info = match reqwest::get(format!(
                  "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
                  username.as_ref().unwrap(),
                  math::hexdigest(hash)
                )).await {
                  Ok(v) => {
                    info!("got status code: {}", v.status());
                    if v.status() == StatusCode::NO_CONTENT {
                      self.send_disconnect("Invalid auth token! Please re-login (restart your game and launcher)").await?;
                      // Disconnect client; they are not authenticated
                      return Ok(None);
                    }
                    match v.json().await {
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

                self.send_compression(compression).await?;
                self.send_success(info.as_ref().unwrap()).await?;
                // Successful login, we can break now
                break 'login;
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
      }
    }
    Ok(info)
  }
}

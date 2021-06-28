mod version;

use crate::settings::{AccountInfo, Settings};
use common::{
  math,
  math::der,
  net::{sb, tcp},
  stream::{
    java::{self, JavaStreamReader, JavaStreamWriter},
    StreamReader, StreamWriter,
  },
  version::ProtocolVersion,
};
use rand::{rngs::OsRng, RngCore};
use reqwest::StatusCode;
use rsa::{PaddingScheme, PublicKey};
use serde_derive::Serialize;
use sha1::{Digest, Sha1};
use std::{error::Error, io, io::ErrorKind, sync::Mutex};
use tokio::net::TcpStream;
use version::Generator;

#[derive(Debug, Copy, Clone)]
pub enum State {
  Handshake,
  Status,
  Login,
  Play,
}

pub struct Connection {
  reader: Mutex<JavaStreamReader>,
  writer: Mutex<JavaStreamWriter>,
  ver:    ProtocolVersion,
  state:  State,
  gen:    Generator,
}

#[derive(Serialize, Debug)]
struct JoinInfo {
  #[serde(rename = "accessToken")]
  access_token:     String,
  #[serde(rename = "selectedProfile")]
  selected_profile: String, // UUID without dashes
  #[serde(rename = "serverId")]
  server_id:        String,
}

impl Connection {
  pub async fn new(ip: &str, settings: &Settings) -> Option<Self> {
    info!("connecting to {}...", ip);
    let tcp_stream = match TcpStream::connect(ip).await {
      Ok(s) => s,
      Err(e) => {
        error!("could not connect to {}: {}", ip, e);
        return None;
      }
    };

    let (reader, writer) = java::stream::new(tcp_stream).unwrap();
    let mut conn = Connection {
      reader: Mutex::new(reader),
      writer: Mutex::new(writer),
      ver:    ProtocolVersion::V1_8,
      state:  State::Handshake,
      gen:    Generator::new(),
    };
    if let Err(e) = conn.handshake(ip, &settings.get_info()).await {
      error!("could not finish handshake with {}: {}", ip, e);
      return None;
    }

    Some(conn)
  }

  pub async fn run(&self) -> Result<(), Box<dyn Error>> {
    loop {
      self.reader.lock().unwrap().poll().await?;
      loop {
        let p = self.reader.lock().unwrap().read(self.ver).unwrap();
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
        let packets = match self.gen.clientbound(self.ver, p) {
          Ok(p) => p,
          Err(e) => match e.kind() {
            ErrorKind::Other => {
              return Err(Box::new(e));
            }
            _ => {
              warn!("{}", e);
              continue;
            }
          },
        };
        for p in packets {
          match p.id() {
            id => warn!("unknown packet recieved from server: {:?}", id),
          }
        }
      }
    }
  }

  pub async fn send(&self, p: sb::Packet) {}

  async fn handshake(&mut self, ip: &str, info: &AccountInfo) -> Result<(), io::Error> {
    let reader = self.reader.get_mut().unwrap();
    let writer = self.writer.get_mut().unwrap();

    let mut out = tcp::Packet::new(0, self.ver); // Handshake
    out.write_varint(self.ver.id() as i32); // Protocol version
    out.write_str(ip); // Ip
    out.write_u16(25565); // Port
    out.write_varint(2); // Going to login
    writer.write(out).await?;
    self.state = State::Login;

    let mut out = tcp::Packet::new(0, self.ver); // Login start
    out.write_str(info.username());
    writer.write(out).await?;

    'login: loop {
      reader.poll().await.unwrap();
      loop {
        let p = reader.read(self.ver).unwrap();
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
        info!("got packet id: {}", p.id());
        match self.state {
          State::Handshake => unreachable!(),
          State::Status => {
            info!("got status packet: {}", p.id());
          }
          State::Login => {
            match p.id() {
              // Disconnect
              0 => {
                info!("got disconnect packet from server during login");
                return Ok(());
              }
              // Encryption request
              1 => {
                let _server_id = p.read_str();
                let key_len = p.read_varint();
                let der_key = p.read_buf(key_len);
                let token_len = p.read_varint();
                let token = p.read_buf(token_len);

                let key = der::decode(&der_key).ok_or_else(|| {
                  io::Error::new(ErrorKind::InvalidInput, format!("invalid der key"))
                })?;

                let mut rng = OsRng;
                let mut secret = [0; 16];
                rng.fill_bytes(&mut secret);

                let encrypted_secret =
                  key.encrypt(&mut rng, PaddingScheme::PKCS1v15Encrypt, &secret).map_err(|e| {
                    io::Error::new(
                      ErrorKind::InvalidInput,
                      format!("could not encrypt secret: {}", e),
                    )
                  })?;
                let encrypted_token =
                  key.encrypt(&mut rng, PaddingScheme::PKCS1v15Encrypt, &token).map_err(|e| {
                    io::Error::new(
                      ErrorKind::InvalidInput,
                      format!("could not encrypt token: {}", e),
                    )
                  })?;

                let mut hash = Sha1::new();
                hash.update("");
                hash.update(secret);
                hash.update(der_key);
                info!("{:?} {}", &info, info.uuid().as_str());
                let info = JoinInfo {
                  access_token:     info.access_token().into(),
                  selected_profile: info.uuid().as_str(),
                  server_id:        math::hexdigest(hash),
                };
                let client = reqwest::Client::new();
                match client
                  .post("https://sessionserver.mojang.com/session/minecraft/join")
                  .json(&info)
                  .send()
                  .await
                {
                  Ok(res) => {
                    if res.status() != StatusCode::NO_CONTENT {
                      return Err(io::Error::new(
                        ErrorKind::Other,
                        format!("failed to authenticate client: \n{}", res.text().await.unwrap()),
                      ));
                    }
                  }
                  Err(e) => {
                    return Err(io::Error::new(
                      ErrorKind::Other,
                      format!("failed to authenticate client: {}", e),
                    ))
                  }
                }

                let mut out = tcp::Packet::new(1, self.ver); // Encryption response
                out.write_varint(encrypted_secret.len() as i32);
                out.write_buf(&encrypted_secret);
                out.write_varint(encrypted_token.len() as i32);
                out.write_buf(&encrypted_token);
                writer.write(out).await?;

                writer.enable_encryption(&secret);
                reader.enable_encryption(&secret);
              }
              // Login success
              2 => {
                self.state = State::Play;
                info!("successful login");
                break 'login;
              }
              // Set compression
              3 => {
                let level = p.read_varint();
                reader.set_compression(level);
                writer.set_compression(level);
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

    Ok(())
  }
}

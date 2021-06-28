use crate::settings::Settings;
use common::{
  math,
  math::der,
  net::tcp,
  stream::{
    java::{self, JavaStreamReader, JavaStreamWriter},
    StreamReader, StreamWriter,
  },
  version::ProtocolVersion,
};
use rand::{rngs::OsRng, RngCore};
use reqwest::StatusCode;
use rsa::{PaddingScheme, PublicKey};
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{io, io::ErrorKind};
use tokio::net::TcpStream;

#[derive(Debug, Copy, Clone)]
pub enum State {
  Handshake,
  Status,
  Login,
  Play,
  Invalid,
}

pub struct Connection {
  reader: JavaStreamReader,
  writer: JavaStreamWriter,
  ver:    ProtocolVersion,
  state:  State,
}

#[derive(Serialize, Debug)]
struct JoinInfo {
  access_token:     String,
  selected_profile: String, // UUID without dashes
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
    let mut conn =
      Connection { reader, writer, ver: ProtocolVersion::V1_8, state: State::Handshake };
    if let Err(e) = conn.handshake(ip, "macmv").await {
      error!("could not finish handshake with {}: {}", ip, e);
      return None;
    }

    Some(conn)
  }

  async fn handshake(&mut self, ip: &str, name: &str) -> Result<(), io::Error> {
    let mut out = tcp::Packet::new(0, self.ver); // Handshake
    out.write_varint(self.ver.id() as i32); // Protocol version
    out.write_str(ip); // Ip
    out.write_u16(25565); // Port
    out.write_varint(2); // Going to login
    self.writer.write(out).await?;
    self.state = State::Login;

    let mut out = tcp::Packet::new(0, self.ver); // Login start
    out.write_str(name); // Username
    self.writer.write(out).await?;

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
                let info = JoinInfo {
                  access_token:     "".into(),
                  selected_profile: "".into(),
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
                    if res.status() != StatusCode::OK {
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
                self.writer.write(out).await?;

                self.writer.enable_encryption(&secret);
                self.reader.enable_encryption(&secret);

                // if username.is_some() {
                //   return Err(io::Error::new(
                //     ErrorKind::InvalidInput,
                //     "client sent two login packets",
                //   ));
                // }
                // let name = p.read_str();
                // username = Some(name.to_string());
                // if der_key.is_none() {
                //   info = Some(LoginInfo {
                //     // Generate uuid if we are in offline mode
                //     id: UUID::from_bytes(*md5::compute(&name)),
                //     name,
                //     properties: vec![],
                //   });
                // }
                //
                // match &der_key {
                //   Some(key) => {
                //     // Make sure to actually generate a token
                //     OsRng.fill_bytes(&mut token);
                //
                //     // Encryption request
                //     let mut out = tcp::Packet::new(1, self.ver);
                //     out.write_str(""); // Server id, should be empty
                //     out.write_varint(key.len() as i32); // Key len
                //     out.write_buf(key); // DER encoded RSA key
                //     out.write_varint(4); // Token len
                //     out.write_buf(&token); // Verify token
                //     self.writer.write(out).await?;
                //     // Wait for encryption response to enable encryption
                //   }
                //   None => {
                //     self.send_compression(compression).await?;
                //     self.send_success(info.as_ref().unwrap()).await?;
                //     // Successful login, we can break now
                //     break 'login;
                //   }
                // }
              }
              // Login success
              2 => {
                // if username.is_none() {
                //   return Err(io::Error::new(
                //     ErrorKind::InvalidInput,
                //     "client did not send login start before sending ecryption response",
                //   ));
                // }
                // let len = p.read_varint();
                // let recieved_secret = p.read_buf(len);
                // let len = p.read_varint();
                // let recieved_token = p.read_buf(len);
                //
                // let decrypted_secret =
                //   key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recieved_secret).unwrap();
                // let decrypted_token =
                //   key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recieved_token).unwrap();
                //
                // // Make sure the client sent the correct verify token back
                // if decrypted_token != token {
                //   return Err(io::Error::new(
                //     ErrorKind::InvalidInput,
                //     format!(
                //       "invalid verify token recieved from client (len: {})",
                //       decrypted_token.len()
                //     ),
                //   ));
                // }
                // let len = decrypted_secret.len();
                // let secret = match decrypted_secret.try_into() {
                //   Ok(v) => v,
                //   Err(_) => {
                //     return Err(io::Error::new(
                //       ErrorKind::InvalidInput,
                //       format!(
                //         "invalid secret recieved from client (len: {}, expected len 16)",
                //         len,
                //       ),
                //     ))
                //   }
                // };
                //
                // let mut hash = Sha1::new();
                // hash.update("");
                // hash.update(secret);
                // hash.update(der_key.unwrap());
                // info = match reqwest::get(format!(
                //   "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
                //   username.as_ref().unwrap(),
                //   hexdigest(hash)
                // )).await {
                //   Ok(v) => match v.json().await {
                //     Ok(v) => Some(v),
                //     Err(e) => return Err(io::Error::new(
                //       ErrorKind::InvalidData,
                //       format!("invalid json data recieved from session server: {}", e),
                //     ))
                //   },
                //   Err(e) => return Err(io::Error::new(
                //     ErrorKind::Other,
                //     format!("failed to authenticate client: {}", e),
                //   ))
                // };
                //
                // self.writer.enable_encryption(&secret);
                // self.reader.enable_encryption(&secret);
                //
                // self.send_compression(compression).await?;
                // self.send_success(info.as_ref().unwrap()).await?;
                // Successful login, we can break now
                info!("successful login");
                break 'login;
              }
              // Set compression
              3 => {
                let level = p.read_varint();
                self.reader.set_compression(level);
                self.writer.set_compression(level);
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

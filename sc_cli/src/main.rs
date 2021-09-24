#[macro_use]
extern crate log;

mod handle;

use rand::{rngs::OsRng, Rng};
use rsa::PublicKey;
use sc_common::{
  math::der,
  net::{cb, sb, tcp},
  version::ProtocolVersion,
};
use sc_proxy::{
  conn::State,
  stream::{
    java::{JavaStreamReader, JavaStreamWriter},
    StreamReader, StreamWriter,
  },
};
use std::{
  error::Error,
  io,
  sync::{Arc, Mutex},
};
use tokio::net::TcpStream;

pub struct ConnWriter {
  stream: JavaStreamWriter,
  ver:    ProtocolVersion,
}
pub struct ConnReader {
  stream: JavaStreamReader,
  ver:    ProtocolVersion,
}

impl ConnWriter {
  pub async fn write(&mut self, p: sb::Packet) -> Result<(), io::Error> {
    self.stream.write(p.to_tcp(self.ver)).await
  }

  pub async fn flush(&mut self) -> Result<(), io::Error> {
    self.stream.flush().await
  }
}
impl ConnReader {
  pub async fn poll(&mut self) -> Result<(), io::Error> {
    self.stream.poll().await
  }

  pub fn read(&mut self) -> Result<Option<cb::Packet>, io::Error> {
    Ok(self.stream.read(self.ver)?.map(|p| cb::Packet::from_tcp(p, self.ver)))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  sc_common::init("cli");

  let ver = ProtocolVersion::V1_8;

  let ip = "127.0.0.1:25565";
  info!("connecting to {}", ip);
  let stream = TcpStream::connect(ip).await?;
  info!("connection established");

  let (read, write) = stream.into_split();
  let mut reader = JavaStreamReader::new(read);
  let mut writer = JavaStreamWriter::new(write);

  handshake(&mut reader, &mut writer, ver).await?;
  info!("login complete");

  let reader = ConnReader { stream: reader, ver };
  let writer = Arc::new(Mutex::new(ConnWriter { stream: writer, ver }));

  let mut handler = handle::Handler { reader, writer: writer.clone() };
  handler.run().await?;

  info!("closing");

  Ok(())
}

async fn handshake(
  reader: &mut JavaStreamReader,
  writer: &mut JavaStreamWriter,
  ver: ProtocolVersion,
) -> Result<(), Box<dyn Error>> {
  let mut out = tcp::Packet::new(0, ver);
  out.write_varint(ProtocolVersion::V1_8.id() as i32);
  out.write_str("127.0.0.1");
  out.write_u16(25565);
  out.write_varint(2); // login state
  writer.write(out).await?;
  let mut out = tcp::Packet::new(0, ProtocolVersion::V1_8);
  out.write_str("macmv");
  writer.write(out).await?;
  writer.flush().await?;

  let state = State::Login;

  loop {
    reader.poll().await?;

    loop {
      let mut p = match reader.read(ProtocolVersion::V1_8)? {
        None => break,
        Some(p) => p,
      };
      match state {
        State::Handshake => unreachable!(),
        State::Status => unreachable!(),
        State::Login => match p.id() {
          0 => {
            // disconnect
            let reason = p.read_str();
            error!("disconnected: {}", reason);
            return Ok(());
          }
          1 => {
            // encryption request
            warn!("got encryption request, but mojang auth is not implemented");

            let _server_id = p.read_str();
            let pub_key_len = p.read_varint();
            let pub_key = p.read_buf(pub_key_len);
            let token_len = p.read_varint();
            let token = p.read_buf(token_len);
            let key = der::decode(&pub_key).unwrap();

            let mut secret = [0; 16];
            let mut rng = OsRng;
            rng.fill(&mut secret);

            let enc_secret = key.encrypt(&mut rng, rsa::PaddingScheme::PKCS1v15Encrypt, &secret)?;
            let enc_token = key.encrypt(&mut rng, rsa::PaddingScheme::PKCS1v15Encrypt, &token)?;

            let mut out = tcp::Packet::new(1, ProtocolVersion::V1_8);
            out.write_varint(enc_secret.len() as i32);
            out.write_buf(&enc_secret);
            out.write_varint(enc_token.len() as i32);
            out.write_buf(&enc_token);
            writer.write(out).await?;
            writer.flush().await?;

            reader.enable_encryption(&secret);
            writer.enable_encryption(&secret);
          }
          2 => {
            // login success
            let _uuid = p.read_uuid();
            let _username = p.read_str();
            return Ok(());
          }
          3 => {
            // set compression
            let thresh = p.read_varint();
            writer.set_compression(thresh);
            reader.set_compression(thresh);
          }
          _ => unreachable!(),
        },
        State::Play => unreachable!(),
        State::Invalid => unreachable!(),
      }
    }
  }
}

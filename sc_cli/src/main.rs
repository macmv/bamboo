#[macro_use]
extern crate log;

use sc_common::{
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
use std::error::Error;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  sc_common::init("cli");

  let ip = "127.0.0.1:25565";
  info!("connecting to {}", ip);
  let stream = TcpStream::connect(ip).await?;
  info!("connection established");

  let (read, write) = stream.into_split();
  let mut reader = JavaStreamReader::new(read);
  let mut writer = JavaStreamWriter::new(write);

  handshake(&mut reader, &mut writer).await?;

  loop {}
}

async fn handshake(
  reader: &mut JavaStreamReader,
  writer: &mut JavaStreamWriter,
) -> Result<(), Box<dyn Error>> {
  let mut out = tcp::Packet::new(0, ProtocolVersion::V1_8);
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
            warn!("disconnected: {}", reason);
            return Ok(());
          }
          1 => {
            // encryption request
            let _server_id = p.read_str();
            let pub_key_len = p.read_varint();
            let pub_key = p.read_buf(pub_key_len);
            let token_len = p.read_varint();
            let token = p.read_buf(token_len);
            unimplemented!();
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

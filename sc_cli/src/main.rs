#[macro_use]
extern crate log;

use sc_common::{
  net::{cb, sb, tcp},
  version::ProtocolVersion,
};
use sc_proxy::stream::{
  java::{JavaStreamReader, JavaStreamWriter},
  StreamReader, StreamWriter,
};
use std::error::Error;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  sc_common::init("cli");

  let ip = "127.0.0.1:25565";
  info!("connecting to {}", ip);
  let mut stream = TcpStream::connect(ip).await?;
  info!("connection established");

  let (read, write) = stream.into_split();
  let mut reader = JavaStreamReader::new(read);
  let mut writer = JavaStreamWriter::new(write);

  let mut out = tcp::Packet::new(0, ProtocolVersion::V1_8);
  out.write_varint(ProtocolVersion::V1_8.id() as i32);
  out.write_str("127.0.0.1");
  out.write_u16(25565);
  out.write_varint(2);
  writer.write(out).await?;
  let mut out = tcp::Packet::new(0, ProtocolVersion::V1_8);
  out.write_str("macmv");
  writer.write(out).await?;
  writer.flush().await?;

  loop {
    reader.poll().await;

    loop {
      let p = match reader.read(ProtocolVersion::V1_8)? {
        None => break,
        Some(p) => p,
      };
      dbg!(p);
    }
  }
}

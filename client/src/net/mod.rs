use common::{net::tcp, version::ProtocolVersion};
use std::net::IpAddr;
use tokio::net::TcpStream;

mod reader;
mod writer;

use reader::TCPReader;
use writer::TCPWriter;

pub struct Connection {
  reader: TCPReader,
  writer: TCPWriter,
}

impl Connection {
  pub async fn new(ip: &str) -> Option<Self> {
    info!("connecting to {}...", ip);
    let stream = match TcpStream::connect(ip).await {
      Ok(s) => s,
      Err(e) => {
        error!("could not connect to {}: {}", ip, e);
        return None;
      }
    };

    let (read, write) = stream.into_split();
    let conn = Connection { reader: TCPReader::new(read), writer: TCPWriter::new(write) };
    conn.handshake(ip, "macmv").await;

    Some(conn)
  }

  async fn handshake(&self, ip: &str, name: &str) {
    let out = tcp::Packet::new(0, ProtocolVersion::V1_8); // Handshake
    out.write_varint(ProtocolVersion::V1_8.id() as i32); // Protocol version
    out.write_str(ip); // Ip
    out.write_u16(25565); // Port
    out.write_varint(2); // Going to login
    self.writer.send(out).await;

    let out = tcp::Packet::new(0, ProtocolVersion::V1_8); // Login start
    out.write_str(name); // Username
    self.writer.send(out).await;
  }
}

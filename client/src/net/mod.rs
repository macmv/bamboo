use common::{
  net::tcp,
  stream::{
    java::{self, JavaStreamReader, JavaStreamWriter},
    StreamWriter,
  },
  version::ProtocolVersion,
};
use std::io;
use tokio::net::TcpStream;

pub struct Connection {
  reader: JavaStreamReader,
  writer: JavaStreamWriter,
}

impl Connection {
  pub async fn new(ip: &str) -> Option<Self> {
    info!("connecting to {}...", ip);
    let tcp_stream = match TcpStream::connect(ip).await {
      Ok(s) => s,
      Err(e) => {
        error!("could not connect to {}: {}", ip, e);
        return None;
      }
    };

    let (reader, writer) = java::stream::new(tcp_stream).unwrap();
    let mut conn = Connection { reader, writer };
    if let Err(e) = conn.handshake(ip, "macmv").await {
      error!("could not finish handshake with {}: {}", ip, e);
      return None;
    }

    Some(conn)
  }

  async fn handshake(&mut self, ip: &str, name: &str) -> Result<(), io::Error> {
    let mut out = tcp::Packet::new(0, ProtocolVersion::V1_8); // Handshake
    out.write_varint(ProtocolVersion::V1_8.id() as i32); // Protocol version
    out.write_str(ip); // Ip
    out.write_u16(25565); // Port
    out.write_varint(2); // Going to login
    self.writer.write(out).await?;

    let mut out = tcp::Packet::new(0, ProtocolVersion::V1_8); // Login start
    out.write_str(name); // Username
    self.writer.write(out).await?;

    Ok(())
  }
}

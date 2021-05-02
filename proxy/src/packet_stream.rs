use crate::packet::Packet;

use tokio::net::TcpStream;

pub struct Stream {
  stream: TcpStream,
}

impl Stream {
  pub fn new(stream: TcpStream) -> Self {
    Stream { stream }
  }

  pub fn read() -> Packet {
    Packet {}
  }
  pub fn write(p: Packet) {}
}

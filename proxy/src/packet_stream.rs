use crate::packet::Packet;

use tokio::net::TcpStream;

pub struct Stream {
  stream: TcpStream,
}

impl Stream {
  pub fn new(stream: TcpStream) -> Self {
    Stream { stream }
  }

  pub fn read(&self) -> Packet {
    Packet::new(vec![1, 2, 3])
  }
  pub fn write(&self, p: Packet) {}
}

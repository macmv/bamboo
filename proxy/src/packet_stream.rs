use crate::packet::Packet;

use common::util::Buffer;
use std::io::Result;
use tokio::{io::AsyncReadExt, net::TcpStream};

pub struct Stream {
  stream: TcpStream,
  buf: Buffer,
}

impl Stream {
  pub fn new(stream: TcpStream) -> Self {
    Stream { stream, buf: Buffer::new(vec![]) }
  }

  pub async fn poll(&mut self) -> Result<()> {
    let mut vec = vec![0u8; 256];
    let len = self.stream.read(&mut vec).await?;
    vec.truncate(len);
    self.buf.write(vec);
    Ok(())
  }
  pub fn read(&mut self) -> Result<Option<Packet>> {
    let packet_len = self.buf.read_varint() as usize;
    if packet_len > self.buf.len() {
      return Ok(None);
    }
    Ok(Some(Packet::new(self.buf.read(packet_len))))
  }
  pub fn write(&self, p: Packet) {}
}

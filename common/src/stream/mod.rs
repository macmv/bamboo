pub mod bedrock;
pub mod java;

use crate::{net::tcp, version::ProtocolVersion};
use std::io;

#[async_trait]
pub trait StreamReader {
  async fn poll(&mut self) -> io::Result<()> {
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> io::Result<Option<tcp::Packet>>;

  fn enable_encryption(&mut self, _secret: &[u8; 16]) {}
  fn set_compression(&mut self, _level: i32) {}
}
#[async_trait]
pub trait StreamWriter {
  async fn write(&mut self, packet: tcp::Packet) -> io::Result<()>;

  fn enable_encryption(&mut self, _secret: &[u8; 16]) {}
  fn set_compression(&mut self, _level: i32) {}
}

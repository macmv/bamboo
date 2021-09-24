pub mod bedrock;
pub mod java;

use sc_common::{net::tcp, version::ProtocolVersion};
use std::io;
use tokio::time::Duration;
use tonic::async_trait;

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

  /// Returns the amount of time needed before a flush should happen. This is
  /// should be something along the lines of (50 millis - time since last
  /// flush). If this returns None, then the stream does not have any data to
  /// flush.
  fn flush_time(&self) -> Option<Duration> {
    None
  }
  /// Flushes this writer. This will send all internal data to the client, if
  /// there is any stored.
  async fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

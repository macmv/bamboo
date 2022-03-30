// pub mod bedrock;
pub mod java;

use crate::{gnet::tcp, Result};
use bb_common::version::ProtocolVersion;

pub trait PacketStream {
  // Config functions

  /// Enables encryption for this stream. All incoming and outgoing packets will
  /// be encrypted using the given secret.
  fn enable_encryption(&mut self, secret: &[u8; 16]);
  /// Sets the compression level for this stream. A negative level will disable
  /// encryption.
  fn set_compression(&mut self, level: i32);

  // Reading functions

  /// Trys to read bytes from the internal socket. This may return an error of
  /// kind WouldBlock, in which case there isn't any more data left to read
  /// right now.
  fn poll(&mut self) -> Result<()>;
  /// Reads data from the internal buffer, and produces a packet. This will not
  /// read from the tcp stream at all. If Ok(None) is produced, then there
  /// aren't any more packets left in the internal buffer. If Ok(Some(p)) is
  /// returned, then you should continue calling read until Ok(None) is
  /// returned. If an error occures, then the stream is invalid, and the
  /// connection should be terminated.
  fn read(&mut self, ver: ProtocolVersion) -> Result<Option<tcp::Packet>>;

  // Writing functions

  /// Writes the given packet to the internal outgoing buffer. This will never
  /// call flush, and in turn will not interact with the tcp stream at all.
  fn write(&mut self, packet: tcp::Packet);

  /// Returns if this stream needs to be flushed. This should simply return true
  /// if there is any data waiting to be flushed.
  fn needs_flush(&self) -> bool;
  /// Flushes this writer. This will send all internal data to the client, if
  /// there is any stored.
  ///
  /// This may return an error of kind WouldBlock. If this happens, then this
  /// stream still needs to be flushed. You should poll for `Interest::WRITABLE`
  /// and try again.
  fn flush(&mut self) -> Result<()> { Ok(()) }
}

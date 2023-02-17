use crate::stream::{StreamReader, StreamWriter};
use ringbuf::Consumer;
use bb_common::{net::tcp, util::Buffer, version::ProtocolVersion};
use std::{
  io::{self, ErrorKind},
  net::SocketAddr,
  sync::Arc,
};
use tokio::net::UdpSocket;

const MAGIC: &'static [u8] =
  &[0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78];

pub struct BedrockStreamReader {
  cons: Consumer<u8>,
}

pub struct BedrockStreamWriter {
  _sock: Arc<UdpSocket>,
  _addr: SocketAddr,
}

impl BedrockStreamReader {
  pub fn new(cons: Consumer<u8>) -> Self {
    BedrockStreamReader { cons }
  }
}

impl BedrockStreamWriter {
  pub fn new(sock: Arc<UdpSocket>, addr: SocketAddr) -> Self {
    BedrockStreamWriter { _sock: sock, _addr: addr }
  }
}

#[async_trait]
impl StreamWriter for BedrockStreamWriter {
  async fn write(&mut self, _packet: tcp::Packet) -> io::Result<()> {
    Ok(())
  }
}
#[async_trait]
impl StreamReader for BedrockStreamReader {
  fn read(&mut self, _ver: ProtocolVersion) -> io::Result<Option<tcp::Packet>> {
    info!("waiting for data...");

    let id = self.cons.get(0).copied().unwrap_or(0);
    self.cons.discard(1);

    let result = match id {
      // Unconnected Ping
      1 | 2 => {
        // Contains a long, MAGIC, and the client GUID
        let data = self.cons.pop_slice(8 + MAGIC.len() + 8);
        let buf = Buffer::new(data);

        let time = buf.read_u64(); // time in millis
        buf.expect(MAGIC);
        let guid = buf.read_u64(); // client's guid
        info!("got time/guid: {} {}", time, guid);
      }
      _ => {
        return Err(io::Error::new(ErrorKind::InvalidData, format!("Unknown packet id: {}", id)))
      }
    };
    info!("got packet id: {}", id);
    Ok(None)
  }
}

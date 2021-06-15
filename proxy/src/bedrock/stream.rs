use crate::{packet::Packet, StreamReader, StreamWriter};
use common::version::ProtocolVersion;
use std::{
  io,
  net::{SocketAddr, UdpSocket},
  sync::Arc,
};

pub struct BedrockStreamReader {
  sock: Arc<UdpSocket>,
  addr: SocketAddr,
}

pub struct BedrockStreamWriter {
  sock: Arc<UdpSocket>,
  addr: SocketAddr,
}

impl BedrockStreamReader {
  pub fn new(sock: Arc<UdpSocket>, addr: SocketAddr) -> Self {
    BedrockStreamReader { sock, addr }
  }
}

impl BedrockStreamWriter {
  pub fn new(sock: Arc<UdpSocket>, addr: SocketAddr) -> Self {
    BedrockStreamWriter { sock, addr }
  }
}

#[async_trait]
impl StreamWriter for BedrockStreamWriter {
  async fn write(&mut self, packet: Packet) -> io::Result<()> {
    Ok(())
  }
}
#[async_trait]
impl StreamReader for BedrockStreamReader {
  async fn poll(&mut self) -> io::Result<()> {
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> io::Result<Option<Packet>> {
    Ok(None)
  }
}

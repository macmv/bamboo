use crate::{packet::Packet, StreamReader, StreamWriter};
use common::version::ProtocolVersion;
use std::{
  io,
  net::SocketAddr,
  sync::{mpsc::Receiver, Arc},
};
use tokio::net::UdpSocket;

pub struct BedrockStreamReader {
  rx: Receiver<Vec<u8>>,
}

pub struct BedrockStreamWriter {
  sock: Arc<UdpSocket>,
  addr: SocketAddr,
}

impl BedrockStreamReader {
  pub fn new(rx: Receiver<Vec<u8>>) -> Self {
    BedrockStreamReader { rx }
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
    info!("waiting for data...");
    let data = self.rx.recv().unwrap();
    info!("got data: {:?}", data);
    Ok(None)
  }
}

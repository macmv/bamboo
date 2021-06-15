use crate::{packet::Packet, StreamReader, StreamWriter};
use common::{util::Buffer, version::ProtocolVersion};
use ringbuf::{Consumer, Producer, RingBuffer};
use std::{
  io::{self, ErrorKind},
  net::SocketAddr,
  sync::{mpsc::Receiver, Arc},
};
use tokio::net::UdpSocket;

const MAGIC: &'static [u8] =
  &[0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78];

pub struct BedrockStreamReader {
  rx:   Receiver<Vec<u8>>,
  prod: Producer<u8>,
  cons: Consumer<u8>,
}

pub struct BedrockStreamWriter {
  sock: Arc<UdpSocket>,
  addr: SocketAddr,
}

impl BedrockStreamReader {
  pub fn new(rx: Receiver<Vec<u8>>) -> Self {
    let buf = RingBuffer::new(1024);
    let (prod, cons) = buf.split();
    BedrockStreamReader { rx, prod, cons }
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
    let msg = self.rx.recv().unwrap();
    self.prod.push_slice(&msg);
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> io::Result<Option<Packet>> {
    info!("waiting for data...");
    let mut id = 0;
    self.cons.access(|a, _| {
      id = *a.get(0).unwrap_or(&0);
    });
    self.cons.discard(1);
    match id {
      // Unconnected Ping
      1 | 2 => {
        // Contains a long, MAGIC, and the client GUID
        let mut data = vec![0; 8 + MAGIC.len() + 8];
        self.cons.pop_slice(&mut data);
        let mut buf = Buffer::new(data);

        let time = buf.read_u64(); // time in millis
        buf.expect(MAGIC);
        let guid = buf.read_u64(); // client's guid
        info!("got time/guid: {} {}", time, guid);
      }
      _ => {
        return Err(io::Error::new(ErrorKind::InvalidData, format!("Unknown packet id: {}", id)))
      }
    }
    info!("got packet id: {}", id);
    Ok(None)
  }
}

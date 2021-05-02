use crate::packet::Packet;

use common::util;
use ringbuf::{Consumer, Producer, RingBuffer};
use std::io::Result;
use tokio::{io::AsyncReadExt, net::TcpStream};

pub struct Stream {
  stream: TcpStream,
  prod: Producer<u8>,
  cons: Consumer<u8>,
}

impl Stream {
  pub fn new(stream: TcpStream) -> Self {
    let buf = RingBuffer::new(1024);
    let (prod, cons) = buf.split();
    Stream { stream, prod, cons }
  }

  pub async fn poll(&mut self) -> Result<()> {
    let mut vec = vec![0u8; 256];
    let len = self.stream.read(&mut vec).await?;
    vec.truncate(len);
    self.prod.push_slice(&vec);
    Ok(())
  }
  pub fn read(&mut self) -> Result<Option<Packet>> {
    let mut packet_len = 0;
    let mut read = -1;
    self.cons.access(|a, b| {
      let (len, amount_read) = util::read_varint(a);
      packet_len = len;
      read = amount_read;
    });
    if read < 0 {
      // TODO: -1 means invalid varint, so we should error here
      return Ok(None);
    }
    // Incomplete varint, or an incomplete packet
    if read == 0 || packet_len as isize > self.cons.len() as isize {
      return Ok(None);
    }
    // Now that we know we have a valid packet, we pop the length bytes
    self.cons.discard(read as usize);
    let mut vec = vec![0u8; packet_len as usize];
    self.cons.pop_slice(&mut vec);
    Ok(Some(Packet::new(vec)))
  }
  pub fn write(&self, p: Packet) {}
}

use crate::packet::Packet;

use common::{util, util::Buffer};
use ringbuf::{Consumer, Producer, RingBuffer};
use std::{
  io,
  io::{ErrorKind, Result},
  net::TcpStream as StdTcpStream,
};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
  },
};

pub struct StreamReader {
  stream: OwnedReadHalf,
  prod:   Producer<u8>,
  cons:   Consumer<u8>,
}
pub struct StreamWriter {
  stream: OwnedWriteHalf,
}

pub fn new(stream: StdTcpStream) -> Result<(StreamReader, StreamWriter)> {
  // We want to block on read calls
  // stream.set_nonblocking(true)?;
  let (read, write) = TcpStream::from_std(stream)?.into_split();
  Ok((StreamReader::new(read), StreamWriter::new(write)))
}

impl StreamReader {
  pub fn new(stream: OwnedReadHalf) -> Self {
    let buf = RingBuffer::new(1024);
    let (prod, cons) = buf.split();
    StreamReader { stream, prod, cons }
  }

  pub async fn poll(&mut self) -> Result<()> {
    let mut msg = vec![];

    // This appends to msg, so we don't need to truncate
    let n = self.stream.read_buf(&mut msg).await?;
    if n == 0 {
      return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("client has disconnected")));
    }
    self.prod.push_slice(&msg);
    Ok(())
  }
  pub fn read(&mut self) -> Result<Option<Packet>> {
    let mut len = 0;
    let mut read = -1;
    self.cons.access(|a, _| {
      let (a, b) = util::read_varint(a);
      len = a as isize;
      read = b;
    });
    // Varint that is more than 5 bytes long.
    if read < 0 {
      return Err(io::Error::new(ErrorKind::InvalidInput, format!("invalid varint")));
    }
    // Incomplete varint, or an incomplete packet
    if read == 0 || len > self.cons.len() as isize {
      return Ok(None);
    }
    // Now that we know we have a valid packet, we pop the length bytes
    self.cons.discard(read as usize);
    // Now we pop the packet data
    let mut vec = vec![0; len as usize];
    self.cons.pop_slice(&mut vec);
    // And parse it
    Ok(Some(Packet::from_buf(vec)))
  }
}

impl StreamWriter {
  pub fn new(stream: OwnedWriteHalf) -> Self {
    StreamWriter { stream }
  }
  pub async fn write(&mut self, p: Packet) -> Result<()> {
    // This is the packet, including it's id
    let bytes = p.buf.into_inner();

    // Length varint
    let mut buf = Buffer::new(vec![]);
    buf.write_varint(bytes.len() as i32);
    self.stream.write(&buf.into_inner()).await?;

    self.stream.write(&bytes).await?;
    Ok(())
  }
}

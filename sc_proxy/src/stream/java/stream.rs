use super::super::{StreamReader, StreamWriter};
use aes::{
  cipher::{AsyncStreamCipher, NewCipher},
  Aes128,
};
use cfb8::Cfb8;
use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib};
use ringbuf::{Consumer, Producer, RingBuffer};
use sc_common::{net::tcp, util, util::Buffer, version::ProtocolVersion};
use std::{
  io,
  io::{ErrorKind, Read, Result, Write},
  net::TcpStream,
};
use tokio::time::{Duration, Instant};

const FLUSH_SIZE: usize = 16 * 1024;
const FLUSH_TIME: Duration = Duration::from_millis(50);

pub struct JavaStreamReader {
  stream:      TcpStream,
  prod:        Producer<u8>,
  cons:        Consumer<u8>,
  // If this is zero, compression is disabled.
  compression: usize,
  // If this is none, then encryption is disabled.
  cipher:      Option<Cfb8<Aes128>>,
}
pub struct JavaStreamWriter {
  stream:      TcpStream,
  outgoing:    Vec<u8>,
  last_flush:  Instant,
  // If this is zero, compression is disabled.
  compression: usize,
  // If this is none, then encryption is disabled.
  cipher:      Option<Cfb8<Aes128>>,
}

pub fn new(stream: TcpStream) -> Result<(JavaStreamReader, JavaStreamWriter)> {
  // We want to block on read calls
  // stream.set_nonblocking(true)?;
  let s = stream.try_clone()?;
  Ok((JavaStreamReader::new(s), JavaStreamWriter::new(stream)))
}

impl JavaStreamReader {
  pub fn new(stream: TcpStream) -> Self {
    let buf = RingBuffer::new(64 * 1024);
    let (prod, cons) = buf.split();
    JavaStreamReader { stream, prod, cons, compression: 0, cipher: None }
  }
}

#[async_trait]
impl StreamReader for JavaStreamReader {
  async fn poll(&mut self) -> Result<()> {
    let mut msg: &mut [u8] = &mut [0; 1024];

    // This appends to msg, so we don't need to truncate
    info!("reading from stream...");
    let n = self.stream.read(msg)?;
    info!("read {} bytes", n);
    if n == 0 {
      return Err(io::Error::new(ErrorKind::ConnectionAborted, "client has disconnected"));
    } else {
      msg = &mut msg[..n];
    }
    if let Some(c) = &mut self.cipher {
      c.decrypt(msg);
    }
    self.prod.push_slice(msg);
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> Result<Option<tcp::Packet>> {
    let mut len = 0;
    let mut read = -1;
    self.cons.access(|left, right| {
      let mut bytes: &mut [u8] = &mut [0; 5];
      let mut on_left = true;
      for i in 0..5 {
        if on_left {
          match left.get(i) {
            Some(b) => bytes[i] = *b,
            None => on_left = false,
          }
        }
        if !on_left {
          match right.get(i - left.len()) {
            Some(b) => bytes[i] = *b,
            None => {
              bytes = &mut bytes[..i];
              break;
            }
          }
        }
      }
      let (a, b) = util::read_varint(bytes);
      len = a as isize;
      read = b;
    });
    // Varint that is more than 5 bytes long.
    if read < 0 {
      return Err(io::Error::new(ErrorKind::InvalidData, "invalid varint"));
    }
    // Incomplete varint, or an incomplete packet
    if read == 0 || (self.cons.len() as isize) < len + read {
      return Ok(None);
    }
    // Now that we know we have a valid packet, we pop the length bytes
    self.cons.discard(read as usize);
    let mut vec = vec![0; len as usize];
    self.cons.pop_slice(&mut vec);
    // And parse it
    if self.compression != 0 {
      let mut buf = Buffer::new(vec);
      let uncompressed_length = buf.read_varint();
      if uncompressed_length == 0 {
        Ok(Some(tcp::Packet::from_buf(buf.read_all(), ver)))
      } else {
        let decompressed = decompress_to_vec_zlib(&buf.read_all()).map_err(|e| {
          io::Error::new(ErrorKind::InvalidData, format!("invalid zlib data: {:?}", e))
        })?;
        Ok(Some(tcp::Packet::from_buf(decompressed, ver)))
      }
    } else {
      Ok(Some(tcp::Packet::from_buf(vec, ver)))
    }
  }

  fn set_compression(&mut self, compression: i32) {
    self.compression = compression as usize;
  }
  fn enable_encryption(&mut self, secret: &[u8; 16]) {
    self.cipher = Some(Cfb8::new_from_slices(secret, secret).unwrap());
  }
}

impl JavaStreamWriter {
  pub fn new(stream: TcpStream) -> Self {
    JavaStreamWriter {
      stream,
      outgoing: Vec::with_capacity(1024),
      last_flush: Instant::now(),
      compression: 0,
      cipher: None,
    }
  }

  async fn write_data(&mut self, data: &mut [u8]) -> Result<()> {
    if let Some(c) = &mut self.cipher {
      c.encrypt(data);
    }
    if self.outgoing.len() + data.len() > FLUSH_SIZE {
      self.flush().await?;
    }

    self.outgoing.extend(data.iter());
    Ok(())
  }
}

#[async_trait]
impl StreamWriter for JavaStreamWriter {
  async fn write(&mut self, p: tcp::Packet) -> Result<()> {
    // This is the packet, including it's id
    let mut bytes = p.serialize();

    // Either the uncompressed length, or the total and uncompressed length.
    let mut buf = Buffer::new(vec![]);

    if self.compression != 0 {
      if bytes.len() > self.compression {
        let uncompressed_length = bytes.len();
        let mut compressed = compress_to_vec_zlib(&bytes, 1);

        // See how many bytes the uncompressed_length varint takes up
        let mut uncompressed_length_buf = Buffer::new(vec![]);
        uncompressed_length_buf.write_varint(uncompressed_length as i32);

        // This is the total length of the packet.
        let total_length = uncompressed_length_buf.len() + compressed.len();
        buf.write_varint(total_length as i32);
        buf.write_varint(uncompressed_length as i32);
        self.write_data(&mut buf).await?;
        self.write_data(&mut compressed).await?;
      } else {
        // The 1 is for the zero uncompressed_length
        buf.write_varint(bytes.len() as i32 + 1);
        buf.write_varint(0);
        self.write_data(&mut buf).await?;
        self.write_data(&mut bytes).await?;
      }
    } else {
      // Uncompressed packets just have the length prefixed.
      buf.write_varint(bytes.len() as i32);
      self.write_data(&mut buf).await?;
      self.write_data(&mut bytes).await?;
    }

    Ok(())
  }

  fn set_compression(&mut self, compression: i32) {
    self.compression = compression as usize;
  }
  fn enable_encryption(&mut self, secret: &[u8; 16]) {
    self.cipher = Some(Cfb8::new_from_slices(secret, secret).unwrap());
  }

  fn flush_time(&self) -> Option<Duration> {
    if self.outgoing.is_empty() {
      None
    } else {
      Some(
        FLUSH_TIME
          .checked_sub(Instant::now().duration_since(self.last_flush))
          .unwrap_or(Duration::from_millis(0)),
      )
    }
  }

  async fn flush(&mut self) -> Result<()> {
    if self.outgoing.is_empty() {
      return Ok(());
    }
    info!("writing {} bytes", self.outgoing.len());
    self.stream.write(&self.outgoing)?;
    info!("done writing {} bytes", self.outgoing.len());
    // Older clients cannot handle too much data at once. So we literally just slow
    // down their connection when a bunch of data is coming through.
    // if self.outgoing.len() > FLUSH_SIZE / 2 {
    //   time::sleep(Duration::from_millis(16)).await;
    // }
    self.outgoing.clear();
    self.last_flush = Instant::now();
    Ok(())
  }
}

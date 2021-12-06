use super::super::PacketStream;
use aes::{
  cipher::{AsyncStreamCipher, NewCipher},
  Aes128,
};
use cfb8::Cfb8;
use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib};
use mio::net::TcpStream;
use ringbuf::{Consumer, Producer, RingBuffer};
use sc_common::{gnet::tcp, util, util::Buffer, version::ProtocolVersion};
use std::{
  fmt, io,
  io::{ErrorKind, Read, Result, Write},
};

pub struct JavaStream {
  stream: TcpStream,

  recv_prod: Producer<u8>,
  recv_cons: Consumer<u8>,

  outgoing: Vec<u8>,

  // If this is zero, compression is disabled.
  compression: usize,
  // If this is none, then encryption is disabled.
  cipher:      Option<Cfb8<Aes128>>,
}

impl fmt::Debug for JavaStream {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("JavaStream").field("outgoing_len", &self.outgoing.len()).finish()
  }
}

impl JavaStream {
  pub fn new(stream: TcpStream) -> Self {
    let buf = RingBuffer::new(64 * 1024);
    let (recv_prod, recv_cons) = buf.split();
    JavaStream {
      stream,
      recv_prod,
      recv_cons,
      outgoing: Vec::with_capacity(1024),
      compression: 0,
      cipher: None,
    }
  }

  fn write_data(&mut self, data: &mut [u8]) {
    if let Some(c) = &mut self.cipher {
      c.encrypt(data);
    }

    self.outgoing.extend(data.iter());
  }
}

impl PacketStream for JavaStream {
  fn poll(&mut self) -> Result<()> {
    let mut msg: &mut [u8] = &mut [0; 1024];

    // This appends to msg, so we don't need to truncate
    let n = self.stream.read(msg)?;
    if n == 0 {
      return Err(io::Error::new(ErrorKind::ConnectionAborted, "client has disconnected"));
    } else {
      msg = &mut msg[..n];
    }
    if let Some(c) = &mut self.cipher {
      c.decrypt(msg);
    }
    self.recv_prod.push_slice(msg);
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> Result<Option<tcp::Packet>> {
    let mut len = 0;
    let mut read = -1;
    self.recv_cons.access(|left, right| {
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
    if read == 0 || (self.recv_cons.len() as isize) < len + read {
      return Ok(None);
    }
    // Now that we know we have a valid packet, we pop the length bytes
    self.recv_cons.discard(read as usize);
    let mut vec = vec![0; len as usize];
    self.recv_cons.pop_slice(&mut vec);
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

  fn write(&mut self, p: tcp::Packet) {
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
        self.write_data(&mut buf);
        self.write_data(&mut compressed);
      } else {
        // The 1 is for the zero uncompressed_length
        buf.write_varint(bytes.len() as i32 + 1);
        buf.write_varint(0);
        self.write_data(&mut buf);
        self.write_data(&mut bytes);
      }
    } else {
      // Uncompressed packets just have the length prefixed.
      buf.write_varint(bytes.len() as i32);
      self.write_data(&mut buf);
      self.write_data(&mut bytes);
    }
  }

  fn needs_flush(&self) -> bool {
    !self.outgoing.is_empty()
  }

  fn flush(&mut self) -> Result<()> {
    if self.outgoing.is_empty() {
      return Ok(());
    }
    let n = self.stream.write(&self.outgoing)?;
    self.outgoing.drain(0..n);
    Ok(())
  }
}

use super::super::PacketStream;
use crate::{gnet::tcp, Result};
use aes::{
  cipher::{AsyncStreamCipher, NewCipher},
  Aes128,
};
use bb_common::{util, util::Buffer, version::ProtocolVersion};
use cfb8::Cfb8;
use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib};
use mio::net::TcpStream;
use std::{
  collections::VecDeque,
  fmt, io,
  io::{ErrorKind, Read, Write},
};

/// The largest size that an uncompressed or compressed packet can be. Only used
/// when reading packets. This is about 2 mb, and is the same size used in
/// vanilla.
const MAX_PACKET_SIZE: usize = 0x1fffff;

pub struct JavaStream {
  stream: TcpStream,

  recv: VecDeque<u8>,

  outgoing: Vec<u8>,

  // If this is -1, compression is disabled. If this is 0, all packets are compressed.
  compression:  i32,
  // If this is none, then encryption is disabled.
  read_cipher:  Option<Cfb8<Aes128>>,
  write_cipher: Option<Cfb8<Aes128>>,
}

impl fmt::Debug for JavaStream {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("JavaStream").field("outgoing_len", &self.outgoing.len()).finish()
  }
}

impl JavaStream {
  pub fn new(stream: TcpStream) -> Self {
    JavaStream {
      stream,
      recv: VecDeque::new(),
      outgoing: Vec::with_capacity(1024),
      compression: -1,
      read_cipher: None,
      write_cipher: None,
    }
  }

  fn write_data(&mut self, data: &mut [u8]) {
    if let Some(c) = &mut self.write_cipher {
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
      return Err(io::Error::new(ErrorKind::ConnectionAborted, "client has disconnected").into());
    } else {
      msg = &mut msg[..n];
    }
    if let Some(c) = &mut self.read_cipher {
      c.decrypt(msg);
    }
    self.recv.extend(msg.iter());
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> Result<Option<tcp::Packet>> {
    let mut bytes = [0; 5];
    let end = self.recv.len().min(5);
    for (i, b) in self.recv.range(0..end).enumerate() {
      bytes[i] = *b;
    }
    let (len, read) = util::read_varint(&bytes);
    // Varint that is more than 5 bytes long.
    if read < 0 {
      return Err(io::Error::new(ErrorKind::InvalidData, "invalid varint").into());
    }
    let read = read as usize;
    // Incomplete varint
    if read == 0 {
      return Ok(None);
    }
    // Now that we have a valid varint, we make sure the packet isn't too large.
    if len < 0 || len as usize > MAX_PACKET_SIZE {
      // Packet is too long! We want to kick this client now.
      return Err(io::Error::new(ErrorKind::InvalidData, "packet too long").into());
    }
    let len = len as usize;
    // Incomplete packet, but a valid length
    if self.recv.len() < len + read {
      return Ok(None);
    }

    // Now that we know we have a valid packet, we pop the whole packet
    self.recv.drain(0..read);
    let mut vec = Vec::with_capacity(len);
    for v in self.recv.drain(0..len) {
      vec.push(v);
    }
    // And parse it
    if self.compression >= 0 {
      let mut buf = Buffer::new(&mut vec);
      let uncompressed_length = buf.read_varint()?;
      if uncompressed_length < 0 || uncompressed_length as usize > MAX_PACKET_SIZE {
        // Packet is too long! We want to kick this client now.
        return Err(io::Error::new(ErrorKind::InvalidData, "uncompressed packet too long").into());
      }
      if uncompressed_length == 0 {
        Ok(Some(tcp::Packet::from_buf(buf.read_all(), ver)?))
      } else {
        let decompressed = decompress_to_vec_zlib(&buf.read_all()).map_err(|e| {
          io::Error::new(ErrorKind::InvalidData, format!("invalid zlib data: {:?}", e))
        })?;
        Ok(Some(tcp::Packet::from_buf(decompressed, ver)?))
      }
    } else {
      Ok(Some(tcp::Packet::from_buf(vec, ver)?))
    }
  }

  fn set_compression(&mut self, compression: i32) { self.compression = compression; }
  fn enable_encryption(&mut self, secret: &[u8; 16]) {
    self.read_cipher = Some(Cfb8::new_from_slices(secret, secret).unwrap());
    self.write_cipher = Some(Cfb8::new_from_slices(secret, secret).unwrap());
  }

  fn write(&mut self, p: tcp::Packet) {
    // This is the packet, including it's id
    let mut bytes = p.serialize();

    // Either the uncompressed length, or the total and uncompressed length.
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);

    if self.compression >= 0 {
      // as usize won't wrap here, because `self.compression >= 0`
      if bytes.len() > self.compression as usize {
        let uncompressed_length = bytes.len();
        let mut compressed = compress_to_vec_zlib(&bytes, 1);

        // See how many bytes the uncompressed_length varint takes up
        let mut uncompressed_length_data = vec![];
        let mut uncompressed_length_buf = Buffer::new(&mut uncompressed_length_data);
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

  fn needs_flush(&self) -> bool { !self.outgoing.is_empty() }

  fn flush(&mut self) -> Result<()> {
    if self.outgoing.is_empty() {
      return Ok(());
    }
    let n = self.stream.write(&self.outgoing)?;
    self.outgoing.drain(0..n);
    Ok(())
  }
}

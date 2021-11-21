use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::{
  convert::TryFrom,
  error::Error,
  fmt, io,
  io::{Cursor, Read, Write},
  ops::{Deref, DerefMut},
  string::FromUtf8Error,
};

#[derive(Debug)]
pub struct BufferError {
  err:     BufferErrorKind,
  pos:     u64,
  reading: bool,
}

impl fmt::Display for BufferError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.reading {
      write!(f, "error while reading from buffer at index {}: {}", self.pos, self.err)
    } else {
      write!(f, "error while writing to buffer at index {}: {}", self.pos, self.err)
    }
  }
}

#[derive(Debug)]
pub enum BufferErrorKind {
  IO(io::Error),
  FromUtf8Error(FromUtf8Error),
  StringTooLong(u64, u64),
  VarInt(),
}

impl fmt::Display for BufferErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::IO(e) => write!(f, "{}", e),
      Self::FromUtf8Error(e) => write!(f, "{}", e),
      Self::StringTooLong(len, max) => {
        write!(f, "string is {} characters, longer than max {}", len, max)
      }
      Self::VarInt() => write!(f, "varint is too long"),
    }
  }
}

impl Error for BufferError {}
impl Error for BufferErrorKind {}

#[derive(Debug)]
pub struct Buffer {
  data: Cursor<Vec<u8>>,
  err:  Option<BufferError>,
}

macro_rules! add_read {
  ($fn: ident, $ty: ty, $zero: expr) => {
    pub fn $fn(&mut self) -> $ty {
      if self.err.is_some() {
        return $zero;
      }
      match self.data.$fn::<BigEndian>() {
        Ok(v) => v,
        Err(e) => {
          self.set_err(BufferErrorKind::IO(e), true);
          $zero
        }
      }
    }
  };
}
// The same as add_read(), but with no type parameter
macro_rules! add_read_byte {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self) -> $ty {
      if self.err.is_some() {
        return 0;
      }
      match self.data.$fn() {
        Ok(v) => v,
        Err(e) => {
          self.set_err(BufferErrorKind::IO(e), true);
          0
        }
      }
    }
  };
}

macro_rules! add_write {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self, v: $ty) {
      if self.err.is_some() {
        return;
      }
      match self.data.$fn::<BigEndian>(v) {
        Ok(()) => {}
        Err(e) => {
          self.set_err(BufferErrorKind::IO(e), false);
        }
      }
    }
  };
}
// The same as add_read(), but with no type parameter
macro_rules! add_write_byte {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self, v: $ty) {
      if self.err.is_some() {
        return;
      }
      match self.data.$fn(v) {
        Ok(()) => {}
        Err(e) => {
          self.set_err(BufferErrorKind::IO(e), false);
        }
      }
    }
  };
}

impl Buffer {
  pub fn new(data: Vec<u8>) -> Self {
    Buffer { data: Cursor::new(data), err: None }
  }

  pub fn err(&self) -> &Option<BufferError> {
    &self.err
  }
  fn set_err(&mut self, err: BufferErrorKind, reading: bool) {
    self.err = Some(BufferError { err, pos: self.data.position(), reading });
  }

  /// Writes all of data to the buffer. This will increment the position of the
  /// reader as well. Use append to write to the end of the buffer without
  /// changing position.
  pub fn write(&mut self, data: &[u8]) {
    match self.data.write(data) {
      Ok(_) => {}
      Err(e) => self.set_err(BufferErrorKind::IO(e), true),
    };
  }
  pub fn read(&mut self, len: usize) -> Vec<u8> {
    let mut vec = vec![0u8; len];
    match self.data.read(&mut vec) {
      Ok(_) => {}
      Err(e) => self.set_err(BufferErrorKind::IO(e), true),
    }
    vec
  }
  pub fn len(&self) -> usize {
    self.data.get_ref().len()
  }
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
  pub fn index(&self) -> usize {
    usize::try_from(self.data.position()).unwrap()
  }

  pub fn read_bool(&mut self) -> bool {
    self.read_u8() != 0
  }
  add_read_byte!(read_u8, u8);
  add_read!(read_u16, u16, 0);
  add_read!(read_u32, u32, 0);
  add_read!(read_u64, u64, 0);
  add_read_byte!(read_i8, i8);
  add_read!(read_i16, i16, 0);
  add_read!(read_i32, i32, 0);
  add_read!(read_i64, i64, 0);

  add_read!(read_f32, f32, 0.0);
  add_read!(read_f64, f64, 0.0);

  pub fn expect(&mut self, expected: &[u8]) {
    let got = self.read_buf(expected.len());
    if got != expected {
      panic!("expected {:?}, got {:?}", expected, got);
    }
  }

  pub fn write_bool(&mut self, v: bool) {
    if v {
      self.write_u8(1);
    } else {
      self.write_u8(0);
    }
  }
  add_write_byte!(write_u8, u8);
  add_write!(write_u16, u16);
  add_write!(write_u32, u32);
  add_write!(write_u64, u64);
  add_write_byte!(write_i8, i8);
  add_write!(write_i16, i16);
  add_write!(write_i32, i32);
  add_write!(write_i64, i64);

  add_write!(write_f32, f32);
  add_write!(write_f64, f64);

  pub fn read_all(&mut self) -> Vec<u8> {
    if self.err.is_some() {
      return vec![];
    }
    // TODO: Possibly change this limit
    let mut buf = vec![0; 1024];
    match self.data.read_to_end(&mut buf) {
      Ok(len) => buf[buf.len() - len..].to_vec(),
      Err(e) => {
        self.set_err(BufferErrorKind::IO(e), true);
        vec![]
      }
    }
  }

  pub fn read_buf(&mut self, len: usize) -> Vec<u8> {
    if self.err.is_some() {
      return vec![];
    }
    let mut buf = vec![0; len];
    match self.data.read(&mut buf) {
      Ok(_len) => buf.to_vec(),
      Err(e) => {
        self.set_err(BufferErrorKind::IO(e), true);
        vec![]
      }
    }
  }
  pub fn write_buf(&mut self, v: &[u8]) {
    self.data.write_all(v).unwrap();
  }

  /// This writes a fixed point floating number to the buffer. This simply
  /// multiplies the f64 by 32, and then writes that int into the buffer. This
  /// is not used on newer clients, but is common on older clients.
  pub fn write_fixed_int(&mut self, v: f64) {
    self.write_i32((v * 32.0) as i32);
  }

  /// Reads a string. If the length is longer than the given maximum, this will
  /// fail, and return an empty string.
  pub fn read_str(&mut self, max_len: u64) -> String {
    if self.err.is_some() {
      return "".into();
    }
    let len = self.read_varint();
    if len > max_len * 4 {
      self.set_err(BufferErrorKind::StringTooLong(len, max_len), true);
      return "".into();
    }
    let vec = self.read(len as usize);
    match String::from_utf8(vec) {
      Ok(v) => {
        if v.len() > max_len {
          self.set_err(BufferErrorKind::StringTooLong(len, max_len), true);
          "".into();
        } else {
          v
        }
      }
      Err(e) => {
        self.set_err(BufferErrorKind::FromUtf8Error(e), true);
        "".into()
      }
    }
  }
  pub fn write_str(&mut self, v: &str) {
    if self.err.is_some() {
      return;
    }
    self.write_varint(v.len() as i32);
    self.write(v.as_bytes());
  }

  pub fn read_varint(&mut self) -> i32 {
    if self.err.is_some() {
      return 0;
    }
    let mut res: i32 = 0;
    for i in 0..5 {
      let read = self.read_u8();
      if i == 4 && read & 0b10000000 != 0 {
        // TODO: Custom error here
        self.set_err(BufferErrorKind::VarInt(), true);
        return 0;
      }

      let v = read & 0b01111111;
      res |= (v as i32) << (7 * i);

      if read & 0b10000000 == 0 {
        break;
      }
    }
    res
  }
  pub fn write_varint(&mut self, v: i32) {
    // Need to work with u32, as >> acts differently on i32 vs u32.
    let mut val = v as u32;
    for _ in 0..5 {
      let mut b: u8 = val as u8 & 0b01111111;
      val >>= 7;
      if val != 0 {
        b |= 0b10000000;
      }
      self.write_u8(b);
      if val == 0 {
        break;
      }
    }
  }

  pub fn into_inner(self) -> Vec<u8> {
    self.data.into_inner()
  }
}

impl Deref for Buffer {
  type Target = Vec<u8>;

  fn deref(&self) -> &Self::Target {
    self.data.get_ref()
  }
}

impl DerefMut for Buffer {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.data.get_mut()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn read_varint() {
    let mut buf = Buffer::new(vec![1]);
    assert_eq!(1, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![127]);
    assert_eq!(127, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![128, 2]);
    assert_eq!(256, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![255, 255, 255, 255, 15]);
    assert_eq!(-1, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![255, 255, 255, 255, 255]);
    assert_eq!(0, buf.read_varint());
    assert!(buf.err().is_some());
  }

  #[test]
  pub fn write_varint() {
    let mut buf = Buffer::new(vec![]);
    buf.write_varint(1);
    assert!(buf.err().is_none());
    assert_eq!(vec![1], buf.into_inner());

    let mut buf = Buffer::new(vec![]);
    buf.write_varint(127);
    assert!(buf.err().is_none());
    assert_eq!(vec![127], buf.into_inner());

    let mut buf = Buffer::new(vec![]);
    buf.write_varint(256);
    assert!(buf.err().is_none());
    assert_eq!(vec![128, 2], buf.into_inner());

    let mut buf = Buffer::new(vec![]);
    buf.write_varint(-1);
    assert!(buf.err().is_none());
    assert_eq!(vec![255, 255, 255, 255, 15], buf.into_inner());
  }
}

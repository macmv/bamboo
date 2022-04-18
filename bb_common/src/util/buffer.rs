#[cfg(feature = "host")]
use crate::nbt::NBT;
use crate::{math::ChunkPos, nbt::ParseError, util::UUID};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::{
  convert::TryFrom,
  error::Error,
  fmt, io,
  io::{Cursor, Read, Write},
  ops::{Deref, DerefMut},
  string::FromUtf8Error,
};

pub type Result<T> = std::result::Result<T, BufferError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  Reading,
  Writing,
}

use Mode::Reading;

#[derive(Debug)]
pub struct BufferError {
  err:  BufferErrorKind,
  pos:  u64,
  mode: Mode,
}

impl fmt::Display for BufferError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.mode == Mode::Reading {
      write!(f, "error while reading from buffer at index {}: {}", self.pos, self.err)
    } else {
      write!(f, "error while writing to buffer at index {}: {}", self.pos, self.err)
    }
  }
}

#[derive(Debug)]
pub enum BufferErrorKind {
  VarInt,
  IO(io::Error),
  FromUtf8Error(FromUtf8Error),
  StringTooLong { len: u64, max: u64 },
  ArrayTooLong { len: u64, max: u64 },
  NegativeLen(i32),
  Expected(Vec<u8>, Vec<u8>),
  NBT(Box<ParseError>),
}

impl fmt::Display for BufferErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::VarInt => write!(f, "varint is too long"),
      Self::IO(e) => write!(f, "{}", e),
      Self::FromUtf8Error(e) => write!(f, "{}", e),
      Self::StringTooLong { len, max } => {
        write!(f, "string is `{}` characters, longer than max `{}`", len, max)
      }
      Self::ArrayTooLong { len, max } => {
        write!(f, "array is `{}` elements, longer than max `{}`", len, max)
      }
      Self::NegativeLen(len) => write!(f, "len `{}` is negative", len),
      Self::Expected(expected, got) => write!(f, "expected bytes {:x?}, got {:x?}", expected, got),
      Self::NBT(err) => write!(f, "nbt parse error: {:?}", err),
    }
  }
}

impl Error for BufferError {}

impl From<io::Error> for BufferErrorKind {
  fn from(e: io::Error) -> Self { BufferErrorKind::IO(e) }
}
impl From<FromUtf8Error> for BufferErrorKind {
  fn from(e: FromUtf8Error) -> Self { BufferErrorKind::FromUtf8Error(e) }
}
impl From<ParseError> for BufferErrorKind {
  fn from(e: ParseError) -> Self { BufferErrorKind::NBT(Box::new(e)) }
}

#[derive(Debug)]
pub struct Buffer<T> {
  data: Cursor<T>,
}

macro_rules! add_read {
  ($fn: ident, $ty: ty, $zero: expr) => {
    pub fn $fn(&mut self) -> Result<$ty> {
      self.data.$fn::<BigEndian>().map_err(|e| self.err(e, Reading))
    }
  };
}
// The same as add_read(), but with no type parameter
macro_rules! add_read_byte {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self) -> Result<$ty> { self.data.$fn().map_err(|e| self.err(e, Reading)) }
  };
}

macro_rules! add_write {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self, v: $ty) {
      self.data.$fn::<BigEndian>(v).expect("could not write to buffer")
    }
  };
}
// The same as add_write(), but with no type parameter
macro_rules! add_write_byte {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self, v: $ty) { self.data.$fn(v).expect("could not write to buffer") }
  };
}

impl<T> Buffer<T> {
  pub fn new(data: T) -> Self { Buffer { data: Cursor::new(data) } }
  pub fn new_index(data: T, index: usize) -> Self {
    let mut cursor = Cursor::new(data);
    cursor.set_position(index as u64);
    Buffer { data: cursor }
  }

  pub fn err(&self, e: impl Into<BufferErrorKind>, mode: Mode) -> BufferError {
    BufferError { err: e.into(), pos: self.data.position(), mode }
  }

  pub fn into_inner(self) -> T { self.data.into_inner() }
}

impl<T> Buffer<T>
where
  T: AsRef<[u8]>,
{
  pub fn len(&self) -> usize { self.data.get_ref().as_ref().len() }
  pub fn is_empty(&self) -> bool { self.len() == 0 }
  pub fn index(&self) -> usize { usize::try_from(self.data.position()).unwrap() }

  pub fn read_bool(&mut self) -> Result<bool> { Ok(self.read_u8()? != 0) }
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

  /// Expects the next bytes. Returns an error if the next bytes do not match.
  /// This will read the exact number of bytes that are passed in.
  pub fn expect(&mut self, expected: &[u8]) -> Result<()> {
    let got = self.read_buf(expected.len())?;
    if got == expected {
      Ok(())
    } else {
      Err(self.err(BufferErrorKind::Expected(expected.to_vec(), got), Reading))
    }
  }

  /// Doesn't return a result, as this will just return an empty array if we
  /// have read anything.
  pub fn read_all(&mut self) -> Vec<u8> {
    // TODO: Possibly change this limit
    let mut buf = vec![0; 1024];
    match self.data.read_to_end(&mut buf) {
      Ok(len) => buf[buf.len() - len..].to_vec(),
      Err(e) => panic!("failed to read all: {:?}", e),
    }
  }

  pub fn read_buf(&mut self, len: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0; len];
    self.data.read(&mut buf).map_err(|e| self.err(e, Reading))?;
    Ok(buf)
  }

  /// Reads a string. If the length is longer than the given maximum, this will
  /// fail, and return an error.
  pub fn read_str(&mut self, max_len: u64) -> Result<String> {
    let len = self.read_varint()?;
    let len = len.try_into().map_err(|_| self.err(BufferErrorKind::NegativeLen(len), Reading))?;
    if len > max_len * 4 {
      return Err(self.err(BufferErrorKind::StringTooLong { len, max: max_len }, Reading));
    }
    let vec = self.read_buf(len as usize)?;
    match String::from_utf8(vec) {
      Ok(v) => {
        if v.len() > max_len as usize {
          Err(self.err(BufferErrorKind::StringTooLong { len, max: max_len }, Reading))
        } else {
          Ok(v)
        }
      }
      Err(e) => Err(self.err(e, Reading)),
    }
  }

  pub fn read_varint(&mut self) -> Result<i32> {
    let mut res: i32 = 0;
    for i in 0..5 {
      let read = self.read_u8()?;
      if i == 4 && read & 0b10000000 != 0 {
        return Err(self.err(BufferErrorKind::VarInt, Reading));
      }

      let v = read & 0b01111111;
      res |= (v as i32) << (7 * i);

      if read & 0b10000000 == 0 {
        break;
      }
    }
    Ok(res)
  }

  /// Reads a chunk position, as two i32s.
  pub fn read_chunk_pos(&mut self) -> Result<ChunkPos> {
    Ok(ChunkPos::new(self.read_i32()?, self.read_i32()?))
  }

  /// Reads an nbt tag from self.
  #[cfg(feature = "host")]
  pub fn read_nbt(&mut self) -> Result<NBT> {
    NBT::deserialize_buf(self).map_err(|e| self.err(e, Reading))
  }

  /// Reads a length prefixed array of integers.
  pub fn read_i32_arr(&mut self) -> Result<Vec<i32>> {
    let len = self.read_varint()?.try_into().unwrap();
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
      out.push(self.read_i32()?);
    }
    Ok(out)
  }

  /// Reads 16 bytes from the buffer, and returns that as a big endian UUID.
  pub fn read_uuid(&mut self) -> Result<UUID> {
    Ok(UUID::from_be_bytes(self.read_buf(16)?.try_into().unwrap()))
  }

  /// Reads a list from the packet. This is new to 1.17, and simplifies a bunch
  /// of small for loops in previous versions.
  pub fn read_list<U>(&mut self, val: impl Fn(&mut Buffer<T>) -> Result<U>) -> Result<Vec<U>> {
    let len = self.read_varint()?.try_into().unwrap();
    let mut list = Vec::with_capacity(len);
    for _ in 0..len {
      list.push(val(self)?);
    }
    Ok(list)
  }

  /// Reads a list from the packet. If the length is greater than `max`, this
  /// fails. This is new to 1.17, and simplifies a bunch of small for loops in
  /// previous versions.
  pub fn read_list_max<U>(
    &mut self,
    val: impl Fn(&mut Buffer<T>) -> Result<U>,
    max: usize,
  ) -> Result<Vec<U>> {
    let len: usize = self.read_varint()?.try_into().unwrap();
    if len > max {
      return Err(
        self.err(BufferErrorKind::ArrayTooLong { len: len as u64, max: max as u64 }, Reading),
      );
    }
    let mut list = Vec::with_capacity(len);
    for _ in 0..len {
      list.push(val(self)?);
    }
    Ok(list)
  }

  /// Reads a boolean. If true, the closure is called, and the returned value is
  /// wrapped in Some. Otherwise, this returns None.
  pub fn read_option<U>(
    &mut self,
    val: impl FnOnce(&mut Buffer<T>) -> Result<U>,
  ) -> Result<Option<U>> {
    Ok(if self.read_bool()? { Some(val(self)?) } else { None })
  }

  pub fn read_varint_arr(&mut self) -> Result<Vec<i32>> { self.read_list(|buf| buf.read_varint()) }
}

impl<T> Buffer<T>
where
  Cursor<T>: io::Write,
{
  /// Advances the cursor `amount` bytes, without modifying the data.
  pub fn skip(&mut self, amount: u64) {
    let new_pos = self.data.position() + amount;
    self.data.set_position(new_pos);
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

  /// This doesn't return a result, as the only thing that could go wrong is a
  /// oom error, which isn't even returned as an error.
  pub fn write_buf(&mut self, v: &[u8]) { self.data.write_all(v).unwrap(); }

  /// This writes a fixed point floating number to the buffer. This simply
  /// multiplies the f64 by 32, and then writes that int into the buffer. This
  /// is not used on newer clients, but is common on older clients.
  pub fn write_fixed_int(&mut self, v: f64) { self.write_i32((v * 32.0) as i32); }

  pub fn write_str(&mut self, v: &str) {
    self.write_varint(v.len() as i32);
    self.write_buf(v.as_bytes());
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

  /// Writes a chunk position, as two i32s.
  pub fn write_chunk_pos(&mut self, p: ChunkPos) {
    self.write_i32(p.x());
    self.write_i32(p.z());
  }

  pub fn write_i32_arr(&mut self, list: &[i32]) {
    self.write_varint(list.len().try_into().unwrap());
    for v in list {
      self.write_i32(*v);
    }
  }

  /// This writes a UUID into the buffer (in big endian format).
  pub fn write_uuid(&mut self, v: UUID) { self.write_buf(&v.as_be_bytes()); }

  /// Writes a list to the buffer.
  pub fn write_list<U>(&mut self, list: &[U], write: impl Fn(&mut Buffer<T>, &U)) {
    self.write_varint(list.len().try_into().unwrap());
    for v in list {
      write(self, v);
    }
  }

  /// Writes `true` if the option is Some, or `false` if None. If the option is
  /// some, then it also calls the `write` closure.
  pub fn write_option<U>(&mut self, val: &Option<U>, write: impl FnOnce(&mut Buffer<T>, &U)) {
    self.write_bool(val.is_some());
    match val {
      Some(v) => write(self, v),
      None => {}
    }
  }

  pub fn write_varint_arr(&mut self, v: &[i32]) { self.write_list(v, |p, &v| p.write_varint(v)) }
}

impl<T> Deref for Buffer<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { self.data.get_ref() }
}

impl<T> DerefMut for Buffer<T> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.data.get_mut() }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn read_varint() {
    let mut data = vec![1];
    let mut buf = Buffer::new(&mut data);
    assert_eq!(1, buf.read_varint().unwrap());

    let mut data = vec![127];
    let mut buf = Buffer::new(&mut data);
    assert_eq!(127, buf.read_varint().unwrap());

    let mut data = vec![128, 2];
    let mut buf = Buffer::new(&mut data);
    assert_eq!(256, buf.read_varint().unwrap());

    let mut data = vec![255, 255, 255, 255, 15];
    let mut buf = Buffer::new(&mut data);
    assert_eq!(-1, buf.read_varint().unwrap());

    let mut data = vec![255, 255, 255, 255, 255];
    let mut buf = Buffer::new(&mut data);
    assert!(buf.read_varint().is_err());
  }

  #[test]
  pub fn write_varint() {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(1);
    assert_eq!(vec![1], data);

    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(127);
    assert_eq!(vec![127], data);

    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(256);
    assert_eq!(vec![128, 2], data);

    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_varint(-1);
    assert_eq!(vec![255, 255, 255, 255, 15], data);
  }
}

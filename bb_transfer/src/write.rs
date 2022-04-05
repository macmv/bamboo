use super::{zig, Header};

use std::{error::Error, fmt};

type Result = std::result::Result<(), WriteError>;

/// An error in writing. The only possible error is that the internal slice ran
/// out of space.
#[derive(Debug)]
#[non_exhaustive]
pub enum WriteError {
  EOF,
  BufTooLong,
}

impl fmt::Display for WriteError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::EOF => write!(f, "failed to write field: eof reached"),
      Self::BufTooLong => {
        write!(f, "failed to write field: tried to write a buffer that was too large")
      }
    }
  }
}

impl Error for WriteError {}

/// A trait for anything that can be written to a MessageWriter.
pub trait MessageWrite {
  /// Writes `self` into the writer.
  fn write(&self, writer: &mut MessageWriter) -> Result;
}

/// Wrapper around a byte array for writing fields. Every function on this type
/// will write a value that can be read using
/// [`MessageRead`](super::MessageRead).
///
/// See the [crate] level docs for how fields are encoded.
pub struct MessageWriter<'a> {
  data: &'a mut [u8],
  idx:  usize,
}

impl MessageWriter<'_> {
  /// Creates a new MessageWrite. The given slice will be used to write values.
  /// An internal index is used to know where to write. The MessageWrite will
  /// not modify any data past the index it is at. So after writing, you can
  /// call `index`, and know that none of the data past that index has been
  /// modified.
  #[inline(always)]
  pub fn new(data: &mut [u8]) -> MessageWriter { MessageWriter { data, idx: 0 } }

  /// Returns the current index the writer is at. This byte in the internal
  /// slice will not have been modified yet, and will be modified on the next
  /// call to any of the `write_` functions.
  pub fn index(&self) -> usize { self.idx }

  /// Returns true if the writer still has bytes left. If this returns false,
  /// then any future `write_` calls will failed with `WriteError::EOF`.
  pub fn can_write(&self) -> bool { self.idx < self.data.len() }

  /// Writes some generic type T to `self`. Depending on the situation, this
  /// may be easier than calling the individual `write_*` functions. They will
  /// both compile into the same call, so it doesn't matter which function you
  /// use.
  ///
  /// In order to use this is generated code, this needs to use `&T` instead of
  /// `impl Borrow<T>`. This is why everything needs to be passed by reference.
  /// So for writing things like booleans, it will probably look nicer to use
  /// `write_bool(true)` instead of `write(&true)`.
  pub fn write<T>(&mut self, v: &T) -> Result
  where
    T: ?Sized + MessageWrite,
  {
    v.write(self)
  }

  /// Writes the 3 bit header, and 5 bits of extra data. The 3 MSB of `extra`
  /// will be ignored.
  ///
  /// This is private, as the caller can break the state of this reader if they
  /// do not handle the result correctly.
  fn write_header(&mut self, header: Header, mut num: u64) -> Result {
    if num >= 16 {
      num |= 0x10;
    } else {
      num &= !0x10;
    }
    self.write_byte(header.id() | (num as u8) << 3)
  }

  /// Writes a single byte to the buffer. Returns an error if the reader has
  /// written to the entire buffer.
  ///
  /// This is private, as this is doesn't read a `Header`.
  fn write_byte(&mut self, num: u8) -> Result {
    if self.idx >= self.data.len() {
      Err(WriteError::EOF)
    } else {
      self.data[self.idx] = num;
      self.idx += 1;
      Ok(())
    }
  }
  /// Reads a varint from the buffer. If the number is less than 16, then
  /// nothing will be written. This assumes the first 5 bits have already been
  /// written using [`write_header`].
  ///
  /// This is private, as this is doesn't read a `Header`.
  fn write_varint(&mut self, mut v: u64) -> Result {
    if v < 16 {
      return Ok(());
    }
    v >>= 4; // We wrote 5 bits in [`write_header`], which is only 4 bits of `v`.

    loop {
      if v >= 128 {
        self.write_byte(0x80 | v as u8 & !0x80)?;
        v >>= 7;
      } else {
        self.write_byte(v as u8 & !0x80)?;
        break;
      }
    }
    Ok(())
  }
  /// Writes a float to the buffer. This will simply write the 4 bytes of the
  /// float.
  ///
  /// This is private, as it doesn't read a `Header`.
  fn write_float(&mut self, v: f32) -> Result {
    let n = v.to_bits();
    self.write_byte(n as u8)?;
    self.write_byte((n >> 8) as u8)?;
    self.write_byte((n >> 16) as u8)?;
    self.write_byte((n >> 24) as u8)?;
    Ok(())
  }
  /// Writes a double to the buffer. This will simply write the double's 8
  /// bytes.
  ///
  /// This is private, as it doesn't read a `Header`.
  fn write_double(&mut self, v: f64) -> Result {
    let n = v.to_bits();
    self.write_byte(n as u8)?;
    self.write_byte((n >> 8) as u8)?;
    self.write_byte((n >> 16) as u8)?;
    self.write_byte((n >> 24) as u8)?;
    self.write_byte((n >> 32) as u8)?;
    self.write_byte((n >> 40) as u8)?;
    self.write_byte((n >> 48) as u8)?;
    self.write_byte((n >> 56) as u8)?;
    Ok(())
  }

  /// Writes the given number of bytes from the buffer.
  fn write_buf(&mut self, buf: &[u8]) -> Result {
    if self.idx + buf.len() > self.data.len() {
      Err(WriteError::BufTooLong)
    } else {
      self.data[self.idx..self.idx + buf.len()].clone_from_slice(buf);
      self.idx += buf.len();
      Ok(())
    }
  }
}

impl MessageWriter<'_> {
  /// Writes a single boolean to the internal buffer.
  pub fn write_bool(&mut self, v: bool) -> Result {
    self.write_u8(if v { 1 } else { 0 })?;
    Ok(())
  }
  /// Writes a single byte to the internal buffer. Returns an error if the
  /// writer has reached the end of the buffer.
  pub fn write_u8(&mut self, v: u8) -> Result { self.write_u64(v.into()) }
  /// Writes an unsigned 16 bit integer to the internal buffer. Returns an error
  /// if the writer has reached the end of the buffer.
  pub fn write_u16(&mut self, v: u16) -> Result { self.write_u64(v.into()) }
  /// Writes an unsigned 32 bit integer to the internal buffer. Returns an error
  /// if the writer has reached the end of the buffer.
  pub fn write_u32(&mut self, v: u32) -> Result { self.write_u64(v.into()) }
  /// Writes an unsigned 64 bit integer to the internal buffer. Returns an error
  /// if the writer has reached the end of the buffer.
  pub fn write_u64(&mut self, v: u64) -> Result {
    self.write_header(Header::VarInt, v)?;
    self.write_varint(v)
  }
  /// Writes a single signed byte to the internal buffer.
  pub fn write_i8(&mut self, v: i8) -> Result { self.write_u8(zig(v)) }
  /// Writes a signed 16 bit integer to the internal buffer.
  pub fn write_i16(&mut self, v: i16) -> Result { self.write_u16(zig(v)) }
  /// Writes a signed 32 bit integer to the internal buffer. This encodes the
  /// value with zig zag encoding, and then writes that as a u32.
  pub fn write_i32(&mut self, v: i32) -> Result { self.write_u32(zig(v)) }
  /// Writes a signed 64 bit integer to the internal buffer. This encodes the
  /// value with zig zag encoding, and then writes that as a u64.
  pub fn write_i64(&mut self, v: i64) -> Result { self.write_u64(zig(v)) }

  pub fn write_f32(&mut self, v: f32) -> Result {
    self.write_header(Header::Float, 0)?;
    self.write_float(v)
  }
  pub fn write_f64(&mut self, v: f64) -> Result {
    self.write_header(Header::Double, 0)?;
    self.write_double(v)
  }

  pub fn write_str(&mut self, s: &str) -> Result { self.write_bytes(s.as_bytes()) }
  pub fn write_bytes(&mut self, bytes: &[u8]) -> Result {
    self.write_header(Header::Bytes, bytes.len() as u64)?;
    self.write_varint(bytes.len() as u64)?;
    self.write_buf(bytes)
  }

  /// Writes a struct. The number of fields must match the number of write calls
  /// in `writer`. If it does not match, everything past this field in the
  /// message will be invalid.
  pub fn write_struct(
    &mut self,
    num_fields: u64,
    writer: impl FnOnce(&mut MessageWriter) -> Result,
  ) -> Result {
    self.write_header(Header::Struct, num_fields)?;
    self.write_varint(num_fields)?;
    writer(self)
  }
  /// Writes an enum. The variant is some identifier that should be used when
  /// reading to figure out what data is expected. The `num_fields` and `writer`
  /// are passed to [`write_struct`](Self::write_struct).
  pub fn write_enum(
    &mut self,
    variant: u64,
    num_fields: u64,
    writer: impl FnOnce(&mut MessageWriter) -> Result,
  ) -> Result {
    self.write_header(Header::Enum, variant)?;
    self.write_varint(variant)?;
    self.write_struct(num_fields, writer)
  }
  /// Writes a list of type `T`. The length is retrieved from
  /// [`ExactSizeIterator::len`]. Returning an invalid length will generate
  /// invalid data.
  pub fn write_list<T>(&mut self, iter: impl ExactSizeIterator<Item = T>) -> Result
  where
    T: MessageWrite,
  {
    let len = iter.len() as u64;
    self.write_header(Header::List, len)?;
    self.write_varint(len)?;
    for v in iter {
      self.write(&v)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn structs() {
    let mut data = [0; 4];
    let mut m = MessageWriter::new(&mut data);
    m.write_struct(3, |m| {
      m.write_u8(5)?;
      m.write_u8(6)?;
      m.write_u8(7)?;
      Ok(())
    })
    .unwrap();
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(data, [0b100 | 3 << 3, 0b001 | 5 << 3, 0b001 | 6 << 3, 0b001 | 7 << 3]);
  }

  #[test]
  fn enums() {
    let mut data = [0; 5];
    let mut m = MessageWriter::new(&mut data);
    let a = 6_u32;
    m.write_enum(5, 3, |m| {
      m.write(&5_u32)?;
      m.write(&a)?;
      m.write(&7_u32)?;
      Ok(())
    })
    .unwrap();
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(
      data,
      [0b101 | 5 << 3, 0b100 | 3 << 3, 0b001 | 5 << 3, 0b001 | 6 << 3, 0b001 | 7 << 3]
    );
  }

  #[test]
  fn simple() {
    let mut data = [0; 3];
    let mut m = MessageWriter::new(&mut data);
    assert_eq!(m.index(), 0);
    m.write_u8(0).unwrap();
    assert_eq!(m.index(), 1);
    m.write_u8(1).unwrap();
    assert_eq!(m.index(), 2);
    m.write_u8(15).unwrap();
    assert_eq!(m.index(), 3);
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(data, [0b001 | 0 << 3, 0b001 | 1 << 3, 0b001 | 15 << 3]);

    let mut data = [0; 3];
    let mut m = MessageWriter::new(&mut data);
    assert_eq!(m.index(), 0);
    m.write_u8(1).unwrap();
    assert_eq!(m.index(), 1);
    m.write_u8(16).unwrap();
    assert_eq!(m.index(), 3);
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(data, [0b001 | 1 << 3, 0b001 | 0x10 << 3, 1]);
  }

  /*
  #[test]
  fn varints() {
    const EXPECTED: &[u8] = &[
      0,        // 0
      1,        // 1
      127,      // 127
      53 | 128, // ..
      77,       // 53 | 77 << 7
      0,
    ];
    let mut data = [0; EXPECTED.len()];
    let mut m = MessageWriter::new(&mut data);
    assert_eq!(m.index(), 0);
    m.write_u32(0).unwrap();
    assert_eq!(m.index(), 1);
    m.write_u32(1).unwrap();
    assert_eq!(m.index(), 2);
    m.write_u32(127).unwrap();
    assert_eq!(m.index(), 3);
    m.write_u32(53 | 77 << 7).unwrap();
    assert_eq!(m.index(), 5);
    m.write_u32(0).unwrap();
    assert_eq!(m.index(), 6);
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(data, EXPECTED);
  }

  #[test]
  fn bytes() {
    let mut data = [0; 64];
    let mut m = MessageWriter::new(&mut data);
    assert_eq!(m.index(), 0);
    m.write_bytes(b"hello").unwrap();
    assert_eq!(m.index(), 5);
    m.write_bytes(b" world").unwrap();
    assert_eq!(m.index(), 11);
    assert_eq!(&data[..11], b"hello world");

    let mut data = [0; 5];
    let mut m = MessageWriter::new(&mut data);
    assert_eq!(m.index(), 0);
    m.write_bytes(b"hello").unwrap();
    assert_eq!(m.index(), 5);
    assert!(matches!(m.write_bytes(b"a").unwrap_err(), WriteError::BufTooLong));
  }
  */
}

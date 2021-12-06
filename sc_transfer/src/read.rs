use super::zag;

use std::{error::Error, fmt};

type Result<T> = std::result::Result<T, ReadError>;

/// An error while reading a field. This can happen if the end of the internal
/// buffer is reached, or if a varint has too many bytes.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadError {
  /// Varints are encoded such that the highest bit is set to 1 if there is a
  /// byte following, and 0 if the varint has ended. An i32 can only take up to
  /// 5 bytes of space. So, if the highest bit is set on the 5th byte, then we
  /// have an invalid varint, and this error is produced.
  VarIntTooLong,
  /// This happens when reading a buffer (byte array or string) and the length
  /// prefix extends beyond the internal data. This is likely because we aren't
  /// reading the right field, so we should fail.
  InvalidBufLength,
  /// This happens if we read a string, and its not valid UTF8.
  InvalidUTF8,
  /// This happens if we try to read something and there are no bytes left.
  EOF,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::VarIntTooLong => write!(f, "failed to read field: varint was too long"),
      Self::InvalidBufLength => write!(f, "failed to read field: buffer was too long"),
      Self::InvalidUTF8 => write!(f, "failed to read field: invalid utf8 string"),
      Self::EOF => write!(f, "failed to read field: eof reached"),
    }
  }
}

impl Error for ReadError {}

/// A trait for anything that can be read from a MessageReader.
pub trait MessageRead {
  /// Reads a value of Self from the reader.
  fn read(reader: &mut MessageReader) -> Result<Self>
  where
    Self: Sized;
}

/// Wrapper around a byte array for reading fields. Every function on this type
/// will return the same value that was written in the
/// [`MessageWrite`](super::MessageWrite).
///
/// See the [crate] level docs for how fields are decoded.
pub struct MessageReader<'a> {
  data: &'a [u8],
  idx:  usize,
}

impl MessageReader<'_> {
  /// Creates a new MessageReader. This will read data from the given slice, and
  /// use an internal index to know what byte to read from. After reading, you
  /// can call `index`, and know that this will not have read any data past that
  /// index.
  #[inline(always)]
  pub fn new(data: &[u8]) -> MessageReader { MessageReader { data, idx: 0 } }

  /// Returns the current index the reader is at. This byte has not been read,
  /// but will be read the next time any `read_` functions are called.
  pub fn index(&self) -> usize { self.idx }

  /// Returns true if the reader still has bytes left. If this returns false,
  /// then any future `read_` calls will failed with `ReadError::EOF`.
  pub fn can_read(&self) -> bool { self.idx < self.data.len() }

  /// Reads some generic type T from `self`. Depending on the situation, this
  /// may be easier than calling the individual `read_*` functions. They will
  /// both compile into the same call, so it doesn't matter which function you
  /// use.
  pub fn read<T>(&mut self) -> Result<T>
  where
    T: MessageRead,
  {
    T::read(self)
  }

  /// Reads a single boolean from the buffer. Any byte that is non-zero is
  /// interpreted as true. We want to avoid error checking as much as possible,
  /// so it is fine to ignore a value that is not 1 here.
  pub fn read_bool(&mut self) -> Result<bool> { Ok(self.read_u8()? != 0) }
  /// Reads a single byte from the buffer. Returns an error if the reader has
  /// read the entire buffer.
  pub fn read_u8(&mut self) -> Result<u8> {
    if self.idx >= self.data.len() {
      Err(ReadError::EOF)
    } else {
      self.idx += 1;
      Ok(self.data[self.idx - 1])
    }
  }
  /// Reads an unsigned 16 bit integer from the internal buffer. Since 3 bytes
  /// is much larger than 2 bytes, a variable-length integer wouldn't make much
  /// sense. So, this is always encoded as 2 bytes.
  pub fn read_u16(&mut self) -> Result<u16> {
    Ok(self.read_u8()? as u16 | (self.read_u8()? as u16) << 8)
  }
  /// Reads an unsigned 32 bit integer from the internal buffer. 5 bytes is not
  /// much more than 4, so this is encoded as a variable length integer.
  pub fn read_u32(&mut self) -> Result<u32> {
    let mut out = 0;
    let mut i = 0;
    let mut v;
    loop {
      v = self.read_u8()?;
      let done = v & 0x80 == 0;
      out |= ((v as u32) & !0x80) << i * 7;
      if done {
        break;
      }
      i += 1;
      // This is only 5 bytes because 32 / 7 = 4.57
      if i >= 5 {
        return Err(ReadError::VarIntTooLong);
      }
    }
    Ok(out)
  }
  /// Reads an unsigned 64 bit integer from the internal buffer. 10 bytes is not
  /// much more than 8, so this is encoded as a variable length integer.
  pub fn read_u64(&mut self) -> Result<u64> {
    let mut out = 0;
    let mut i = 0;
    let mut v;
    loop {
      v = self.read_u8()?;
      let done = v & 0x80 == 0;
      out |= ((v as u64) & !0x80) << i * 7;
      if done {
        break;
      }
      i += 1;
      // This is not 9 bytes, because 64 / 7 = 9.14, so we need 10 bytes of space
      if i >= 10 {
        return Err(ReadError::VarIntTooLong);
      }
    }
    Ok(out)
  }
  /// Reads a single signed byte from the internal buffer.
  pub fn read_i8(&mut self) -> Result<i8> { Ok(self.read_u8()? as i8) }
  /// Reads a signed 16 bit integer from the internal buffer. Since 3 bytes
  /// is much larger than 2 bytes, a variable-length integer wouldn't make much
  /// sense. So, this is always encoded as 2 bytes.
  pub fn read_i16(&mut self) -> Result<i16> { Ok(self.read_u16()? as i16) }
  /// Reads a signed 32 bit integer from the internal buffer. This reads a u32,
  /// and then decodes it with zig zag encoding.
  pub fn read_i32(&mut self) -> Result<i32> { Ok(zag(self.read_u32()?)) }
  /// Reads a signed 64 bit integer from the internal buffer. This reads a u64,
  /// and then decodes it with zig zag encoding.
  pub fn read_i64(&mut self) -> Result<i64> { Ok(zag(self.read_u64()?)) }
  /// Reads a 32 bit float from the internal buffer. This will always read 4
  /// bytes.
  pub fn read_f32(&mut self) -> Result<f32> {
    let n = self.read_u8()? as u32
      | (self.read_u8()? as u32) << 8
      | (self.read_u8()? as u32) << 16
      | (self.read_u8()? as u32) << 24;
    Ok(f32::from_bits(n))
  }
  /// Reads a 64 bit float from the internal buffer. This will always read 8
  /// bytes.
  pub fn read_f64(&mut self) -> Result<f64> {
    let n = self.read_u8()? as u64
      | (self.read_u8()? as u64) << 8
      | (self.read_u8()? as u64) << 16
      | (self.read_u8()? as u64) << 24
      | (self.read_u8()? as u64) << 32
      | (self.read_u8()? as u64) << 40
      | (self.read_u8()? as u64) << 48
      | (self.read_u8()? as u64) << 56;
    Ok(f64::from_bits(n))
  }
  /// Reads the given number of bytes. This does not write a length prefix.
  pub fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
    if self.idx + len > self.data.len() {
      return Err(ReadError::InvalidBufLength);
    }
    let out = self.data[self.idx..self.idx + len].to_vec();
    self.idx += len;
    Ok(out)
  }
  /// Reads a length prefixed buffer.
  pub fn read_buf(&mut self) -> Result<Vec<u8>> {
    let len = self.read_u32()? as usize;
    self.read_bytes(len)
  }
  /// Reads a length prefixed string.
  pub fn read_str(&mut self) -> Result<String> {
    let buf = self.read_buf()?;
    Ok(String::from_utf8(buf).map_err(|_| ReadError::InvalidUTF8)?)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn simple() {
    let mut m = MessageReader::new(&[0, 0, 2]);
    assert_eq!(m.read_u8().unwrap(), 0);
    assert_eq!(m.read_u8().unwrap(), 0);
    assert_eq!(m.read_u8().unwrap(), 2);
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::EOF));

    let mut m = MessageReader::new(&[127, 0, 0, 1]);
    assert_eq!(m.read_u16().unwrap(), 127);
    assert_eq!(m.read_u16().unwrap(), 256);
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::EOF));
  }

  #[test]
  fn varints() {
    let mut m = MessageReader::new(&[
      0,        // 0
      1,        // 1
      127,      // 127
      53 | 128, // ..
      77,       // 53 | 77 << 7
      0 | 128,  // ..
      0 | 128,  // ..
      0,        // 0 (still valid, because checking for validity like this takes too long)
      0 | 128,  // ..
      0 | 128,  // ..
      0 | 128,  // ..
      0 | 128,  // ..
      0,        // 0 (still valid, because checking for validity like this takes too long)
      0 | 128,  // ..
      0 | 128,  // ..
      0 | 128,  // ..
      0 | 128,  // ..
      0 | 128,  // VarIntTooLong
    ]);
    assert_eq!(m.read_u32().unwrap(), 0);
    assert_eq!(m.read_u32().unwrap(), 1);
    assert_eq!(m.read_u32().unwrap(), 127);
    assert_eq!(m.read_u32().unwrap(), 53 | 77 << 7);
    assert_eq!(m.read_u32().unwrap(), 0);
    assert_eq!(m.read_u32().unwrap(), 0);
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::VarIntTooLong));
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::EOF));
  }

  #[test]
  fn bytes() {
    let mut m = MessageReader::new(b"hello");
    assert_eq!(m.index(), 0);
    assert_eq!(&m.read_bytes(5).unwrap(), b"hello");
    assert_eq!(m.index(), 5);
  }
}

use super::zag;

use std::{error::Error, fmt};

type Result<T> = std::result::Result<T, ReadError>;

/// An error while reading a field. This can happen if the end of the internal
/// buffer is reached, or if a varint has too many bytes.
#[derive(Debug)]
pub enum ReadError {
  VarIntTooLong,
  EOF,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::VarIntTooLong => write!(f, "failed to read field: varint was too long"),
      Self::EOF => write!(f, "failed to read field: eof reached"),
    }
  }
}

impl Error for ReadError {}

/// Wrapper around a byte array for reading fields. Every function on this type
/// will return the same value that was written in the
/// [`MessageWrite`](super::MessageWrite).
///
/// See the [crate] level docs for how fields are decoded.
pub struct MessageRead<'a> {
  data: &'a [u8],
  idx:  usize,
}

impl MessageRead<'_> {
  /// Creates a new MessageRead. This will read data from the given slice, and
  /// use an internal index to know what byte to read from. After reading, you
  /// can call `index`, and know that this will not have read any data past that
  /// index.
  #[inline(always)]
  pub fn new(data: &[u8]) -> MessageRead {
    MessageRead { data, idx: 0 }
  }

  /// Returns the current index the reader is at. This byte has not been read,
  /// but will be read the next time any `read_` functions are called.
  pub fn index(&self) -> usize {
    self.idx
  }

  /// Returns true if the reader still has bytes left. If this returns false,
  /// then any future `read_` calls will failed with `ReadError::EOF`.
  pub fn can_read(&self) -> bool {
    self.idx < self.data.len()
  }

  /// Reads a single boolean from the buffer. Any byte that is non-zero is
  /// interpreted as true. We want to avoid error checking as much as possible,
  /// so it is fine to ignore a value that is not 1 here.
  pub fn read_bool(&mut self) -> Result<bool> {
    Ok(self.read_u8()? != 0)
  }
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
      if i > 4 {
        return Err(ReadError::VarIntTooLong);
      }
    }
    Ok(out)
  }
  /// Reads an unsigned 64 bit integer from the internal buffer. 9 bytes is not
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
      if i > 8 {
        return Err(ReadError::VarIntTooLong);
      }
    }
    Ok(out)
  }
  /// Reads a single signed byte from the internal buffer.
  pub fn read_i8(&mut self) -> Result<i8> {
    Ok(self.read_u8()? as i8)
  }
  /// Reads a signed 16 bit integer from the internal buffer. Since 3 bytes
  /// is much larger than 2 bytes, a variable-length integer wouldn't make much
  /// sense. So, this is always encoded as 2 bytes.
  pub fn read_i16(&mut self) -> Result<i16> {
    Ok(self.read_u16()? as i16)
  }
  /// Reads a signed 32 bit integer from the internal buffer. This reads a u32,
  /// and then decodes it with zig zag encoding.
  pub fn read_i32(&mut self) -> Result<i32> {
    Ok(zag(self.read_u32()?))
  }
  /// Reads a signed 64 bit integer from the internal buffer. This reads a u64,
  /// and then decodes it with zig zag encoding.
  pub fn read_i64(&mut self) -> Result<i64> {
    Ok(zag(self.read_u64()?))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn simple() {
    let mut m = MessageRead::new(&[0, 0, 2]);
    assert_eq!(m.read_u8().unwrap(), 0);
    assert_eq!(m.read_u8().unwrap(), 0);
    assert_eq!(m.read_u8().unwrap(), 2);
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::EOF));

    let mut m = MessageRead::new(&[127, 0, 0, 1]);
    assert_eq!(m.read_u16().unwrap(), 127);
    assert_eq!(m.read_u16().unwrap(), 256);
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::EOF));
  }

  #[test]
  fn varints() {
    let mut m = MessageRead::new(&[
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
}

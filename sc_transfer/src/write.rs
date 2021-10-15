use super::zig;

use std::{error::Error, fmt};

type Result = std::result::Result<(), WriteError>;

/// An error in writing. The only possible error is that the internal slice ran
/// out of space.
#[derive(Debug)]
#[non_exhaustive]
pub enum WriteError {
  EOF,
}

impl fmt::Display for WriteError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::EOF => write!(f, "failed to write field: eof reached"),
    }
  }
}

impl Error for WriteError {}

/// Wrapper around a byte array for writing fields. Every function on this type
/// will write a value that can be read using
/// [`MessageRead`](super::MessageRead).
///
/// See the [crate] level docs for how fields are encoded.
pub struct MessageWrite<'a> {
  data: &'a mut [u8],
  idx:  usize,
}

impl MessageWrite<'_> {
  /// Creates a new MessageWrite. The given slice will be used to write values.
  /// An internal index is used to know where to write. The MessageWrite will
  /// not modify any data past the index it is at. So after writing, you can
  /// call `index`, and know that none of the data past that index has been
  /// modified.
  #[inline(always)]
  pub fn new(data: &mut [u8]) -> MessageWrite {
    MessageWrite { data, idx: 0 }
  }

  /// Returns the current index the writer is at. This byte in the internal
  /// slice will not have been modified yet, and will be modified on the next
  /// call to any of the `write_` functions.
  pub fn index(&self) -> usize {
    self.idx
  }

  /// Returns true if the writer still has bytes left. If this returns false,
  /// then any future `write_` calls will failed with `WriteError::EOF`.
  pub fn can_write(&self) -> bool {
    self.idx < self.data.len()
  }

  /// Writes a single boolean to the internal buffer.
  pub fn write_bool(&mut self, v: bool) -> Result {
    self.write_u8(if v { 1 } else { 0 })?;
    Ok(())
  }
  /// Writes a single byte to the internal buffer. Returns an error if the
  /// writer has reached the end of the buffer.
  pub fn write_u8(&mut self, v: u8) -> Result {
    if self.idx >= self.data.len() {
      Err(WriteError::EOF)
    } else {
      self.data[self.idx] = v;
      self.idx += 1;
      Ok(())
    }
  }
  /// Writes an unsigned 16 bit integer from the internal buffer. Since 3 bytes
  /// is much larger than 2 bytes, a variable-length integer wouldn't make much
  /// sense. So, this is always encoded as 2 bytes.
  pub fn write_u16(&mut self, v: u16) -> Result {
    self.write_u8(v as u8)?;
    self.write_u8((v >> 8) as u8)?;
    Ok(())
  }
  /// Writes an unsigned 32 bit integer from the internal buffer. 5 bytes is not
  /// much more than 4, so this is encoded as a variable length integer (the
  /// smaller the number, then less bytes it uses).
  pub fn write_u32(&mut self, mut v: u32) -> Result {
    loop {
      if v >= 128 {
        self.write_u8(0x80 | v as u8 & !0x80)?;
        v >>= 7;
      } else {
        self.write_u8(v as u8 & !0x80)?;
        break;
      }
    }
    Ok(())
  }
  /// Reads an unsigned 64 bit integer from the internal buffer. 9 bytes is not
  /// much more than 8, so this is encoded as a variable length integer (the
  /// smaller the number, then less bytes it uses).
  pub fn write_u64(&mut self, mut v: u64) -> Result {
    loop {
      if v >= 128 {
        self.write_u8(0x80 | v as u8 & !0x80)?;
        v >>= 7;
      } else {
        self.write_u8(v as u8 & !0x80)?;
        break;
      }
    }
    Ok(())
  }
  /// Writes a single signed byte to the internal buffer.
  pub fn write_i8(&mut self, v: i8) -> Result {
    self.write_u8(v as u8)
  }
  /// Writes a signed 16 bit integer to the internal buffer. Since 3 bytes
  /// is much larger than 2 bytes, a variable-length integer wouldn't make much
  /// sense. So, this is always encoded as 2 bytes.
  pub fn write_i16(&mut self, v: i16) -> Result {
    self.write_u16(v as u16)
  }
  /// Writes a signed 32 bit integer to the internal buffer. This encodes the
  /// value with zig zag encoding, and then writes that as a u32.
  pub fn write_i32(&mut self, v: i32) -> Result {
    self.write_u32(zig(v))
  }
  /// Writes a signed 64 bit integer to the internal buffer. This encodes the
  /// value with zig zag encoding, and then writes that as a u64.
  pub fn write_i64(&mut self, v: i64) -> Result {
    self.write_u64(zig(v))
  }
  /// Writes a 32 bit float to the internal buffer. This will always write 4
  /// bytes.
  pub fn write_f32(&mut self, v: f32) -> Result {
    let n = v.to_bits();
    self.write_u8(n as u8)?;
    self.write_u8((n >> 8) as u8)?;
    self.write_u8((n >> 16) as u8)?;
    self.write_u8((n >> 24) as u8)?;
    Ok(())
  }
  /// Reads a 64 bit float from the internal buffer. This will always read 8
  /// bytes.
  pub fn write_f64(&mut self, v: f64) -> Result {
    let n = v.to_bits();
    self.write_u8(n as u8)?;
    self.write_u8((n >> 8) as u8)?;
    self.write_u8((n >> 16) as u8)?;
    self.write_u8((n >> 24) as u8)?;
    self.write_u8((n >> 32) as u8)?;
    self.write_u8((n >> 40) as u8)?;
    self.write_u8((n >> 48) as u8)?;
    self.write_u8((n >> 56) as u8)?;
    Ok(())
  }
  /// Writes a length prefixed buffer.
  pub fn write_buf(&mut self, v: &[u8]) -> Result {
    self.write_u32(v.len().try_into().expect("capacity overflow"))?;
    self.data[self.idx..self.idx + v.len()].clone_from_slice(v);
    Ok(())
  }
  /// Writes a length prefixed string.
  pub fn write_str(&mut self, v: &str) -> Result {
    self.write_buf(v.as_bytes())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn simple() {
    let mut data = [0; 3];
    let mut m = MessageWrite::new(&mut data);
    m.write_u8(0).unwrap();
    m.write_u8(0).unwrap();
    m.write_u8(2).unwrap();
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(data, [0, 0, 2]);

    let mut data = [0; 4];
    let mut m = MessageWrite::new(&mut data);
    m.write_u16(127).unwrap();
    m.write_u16(256).unwrap();
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(data, [127, 0, 0, 1]);
  }

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
    let mut m = MessageWrite::new(&mut data);
    m.write_u32(0).unwrap();
    m.write_u32(1).unwrap();
    m.write_u32(127).unwrap();
    m.write_u32(53 | 77 << 7).unwrap();
    m.write_u32(0).unwrap();
    assert!(matches!(m.write_u8(5).unwrap_err(), WriteError::EOF));
    assert_eq!(data, EXPECTED);
  }
}

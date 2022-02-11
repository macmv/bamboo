use super::{zag, Header, Message};

use std::{error::Error, fmt, string::FromUtf8Error};

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
  InvalidUtf8(FromUtf8Error),
  /// This happens if the 3 bit header is invalid.
  InvalidHeader(u8),
  /// This happens if we try to read a specific field, and get a different type.
  WrongMessage(Message, Header),
  /// This happens if we try to read something and there are no bytes left.
  EOF,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::VarIntTooLong => write!(f, "failed to read field: varint was too long"),
      Self::InvalidBufLength => write!(f, "failed to read field: buffer was too long"),
      Self::InvalidUtf8(e) => write!(f, "failed to read field: invalid utf8 string: {}", e),
      Self::InvalidHeader(header) => {
        write!(f, "failed to read field: invalid header {header:#x}")
      }
      Self::WrongMessage(m, header) => {
        write!(f, "failed to read field: got message {m:?}, expected message {header:?}")
      }
      Self::EOF => write!(f, "failed to read field: eof reached"),
    }
  }
}

impl From<FromUtf8Error> for ReadError {
  fn from(e: FromUtf8Error) -> Self { ReadError::InvalidUtf8(e) }
}

// TODO:
// Types:
// 0x00 => byte (signed or unsigned)
// 0x01 => int (signed or unsigned), varint encoded
// 0x02 => float
// 0x03 => double
// 0x04 => struct
// 0x05 => enum

impl Error for ReadError {}

/// A trait for anything that can be read from a [`MessageReader`].
pub trait MessageRead {
  /// Reads a value of Self from the reader.
  fn read(reader: &mut MessageReader) -> Result<Self>
  where
    Self: Sized;
}
/// A trait for any struct that can be read from a [`MessageReader`].
pub trait StructRead {
  /// Reads a value of Self from the given struct fields.
  fn read_struct(fields: Vec<Message>) -> Result<Self>
  where
    Self: Sized;
}
/// A trait for any enum that can be read from a [`MessageReader`].
pub trait EnumRead {
  /// Reads a value of Self from the given variant and message.
  fn read_enum(variant: u64, field: Message) -> Result<Self>
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

  /// Reads a 3 bit header for a new field. The `u8` returned is the remaining
  /// bits, shifted right by 3. So this `u8` will only have 5 bits of data set.
  pub fn read_header(&mut self) -> Result<(Header, u8)> {
    let val = self.read_byte()?;
    Ok((Header::from_id(val & 0x07).ok_or(ReadError::InvalidHeader(val & 0x07))?, val >> 3))
  }

  pub fn read_any(&mut self) -> Result<Message> {
    let (header, extra) = self.read_header()?;
    Ok(match header {
      Header::None => Message::None,
      Header::VarInt => Message::VarInt(self.read_varint(extra)?),
      Header::Float => Message::Float(self.read_float()?),
      Header::Double => Message::Double(self.read_double()?),
      Header::Struct => {
        let num_fields = self.read_varint(extra)?;
        Message::Struct(
          (0..num_fields).into_iter().map(|_| self.read_any()).collect::<Result<_>>()?,
        )
      }
      Header::Enum => Message::Enum(self.read_varint(extra)?, Box::new(self.read_any()?)),
      Header::Bytes => {
        let len = self.read_varint(extra)? as usize;
        Message::Bytes(self.read_buf(len)?)
      }
    })
  }

  /// Reads a single byte from the buffer. Returns an error if the reader has
  /// read the entire buffer.
  ///
  /// This is private, as this is doesn't read a `Header`.
  fn read_byte(&mut self) -> Result<u8> {
    if self.idx >= self.data.len() {
      Err(ReadError::EOF)
    } else {
      self.idx += 1;
      Ok(self.data[self.idx - 1])
    }
  }
  /// Reads a varint from the buffer. The given value is a 5 bit LSB header. If
  /// the 5th bit is not set, this will not read anything.
  ///
  /// This is private, as this is doesn't read a `Header`.
  fn read_varint(&mut self, header: u8) -> Result<u64> {
    if header & 0x08 != 0 {
      return Ok(header.into());
    }

    let mut out = header as u64;
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
  /// Reads a float from the buffer. This will simply read 4 bytes, and convert
  /// them into a float.
  ///
  /// This is private, as it doesn't read a `Header`.
  fn read_float(&mut self) -> Result<f32> {
    let n = self.read_u8()? as u32
      | (self.read_u8()? as u32) << 8
      | (self.read_u8()? as u32) << 16
      | (self.read_u8()? as u32) << 24;
    Ok(f32::from_bits(n))
  }
  /// Reads a double from the buffer. This will simply read 8 bytes, and convert
  /// them into a double.
  ///
  /// This is private, as it doesn't read a `Header`.
  fn read_double(&mut self) -> Result<f64> {
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

  /// Reads the given number of bytes from the buffer.
  fn read_buf(&mut self, len: usize) -> Result<Vec<u8>> {
    if self.idx + len > self.data.len() {
      Err(ReadError::InvalidBufLength)
    } else {
      let out = self.data[self.idx..self.idx + len].to_vec();
      self.idx += len;
      Ok(out)
    }
  }
}

macro_rules! read_unsigned {
  ( $reader:ident, $ret:ty ) => {
    /// Reads a field, and makes sure that it is an 8 bit integer.
    ///
    /// Errors:
    /// - If there are no remaining bytes, a [`ReadError::EOF`] is returned.
    /// - If the header read is not a `VarInt`, a [`ReadError::WrongMessage`] is
    ///   returned.
    /// - If the varint parsed is too large, then a [`ReadError::VarIntTooLong`] is
    ///   returned.
    pub fn $reader(&mut self) -> Result<$ret> {
      self.read_any()?.into_varint()?.try_into().map_err(|_| ReadError::VarIntTooLong)
    }
  };
}
macro_rules! read_signed {
  ( $reader:ident, $ret:ty ) => {
    /// Reads a field, and makes sure that it is an 8 bit integer.
    ///
    /// Errors:
    /// - If there are no remaining bytes, a [`ReadError::EOF`] is returned.
    /// - If the header read is not a `VarInt`, a [`ReadError::WrongMessage`] is
    ///   returned.
    /// - If the varint parsed is too large, then a [`ReadError::VarIntTooLong`] is
    ///   returned.
    pub fn $reader(&mut self) -> Result<$ret> {
      self
        .read_any()?
        .into_varint()?
        .try_into()
        .map_err(|_| ReadError::VarIntTooLong)
        .map(|v| zag(v))
    }
  };
}

impl MessageReader<'_> {
  /// Reads a single field. If this is not a `None` field, this returns a
  /// [`ReadError::WrongMessage`] error.
  pub fn read_none(&mut self) -> Result<()> { self.read_any()?.into_none() }

  /// Reads a field. The field must be a `VarInt`, and the value must not be
  /// larger than 1. This field (including the header) will always use 1 byte.
  pub fn read_bool(&mut self) -> Result<bool> {
    let num = self.read_any()?.into_varint()?;
    if num == 0 {
      Ok(false)
    } else if num == 1 {
      Ok(true)
    } else {
      Err(ReadError::VarIntTooLong)
    }
  }

  read_unsigned!(read_u8, u8);
  read_unsigned!(read_u16, u16);
  read_unsigned!(read_u32, u32);
  read_unsigned!(read_u64, u64);

  read_signed!(read_i8, i8);
  read_signed!(read_i16, i16);
  read_signed!(read_i32, i32);
  read_signed!(read_i64, i64);

  /// Reads a float. This will return an error if the header read is not a
  /// `Float` header.
  pub fn read_f32(&mut self) -> Result<f32> { self.read_any()?.into_float() }
  /// Reads a double. This will return an error if the header read is not a
  /// `Double` header.
  pub fn read_f64(&mut self) -> Result<f64> { self.read_any()?.into_double() }

  pub fn read_struct<S: StructRead>(&mut self) -> Result<S> {
    S::read_struct(self.read_any()?.into_struct()?)
  }
  pub fn read_enum<E: EnumRead>(&mut self) -> Result<E> {
    let (variant, field) = self.read_any()?.into_enum()?;
    E::read_enum(variant, field)
  }
  /// Reads a byte array. If the header is not a `Bytes` header, this will
  /// return an error.
  pub fn read_bytes(&mut self) -> Result<Vec<u8>> { self.read_any()?.into_bytes() }
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
    assert_eq!(&m.read_bytes().unwrap(), b"hello");
    assert_eq!(m.index(), 5);
  }
}

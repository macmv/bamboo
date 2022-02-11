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
  /// Returned if an enum variant is invalid.
  InvalidVariant(u64),
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
      Self::InvalidVariant(variant) => {
        write!(f, "failed to read field: invalid variant: {variant}")
      }
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
  fn read_struct(reader: &mut StructReader) -> Result<Self>
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

/// Wrapper around a partially parsed struct. This will validate that all fields
/// were read. This makes it very easy to derive `StructRead` on a struct type.
///
/// This has a single very useful function: [`read`](StructReader::read). This
/// function will read a single field, given an index. The index must be greater
/// than the previous field. If it is two or more indices ahead, this will read
/// `None` fields as placeholders.
///
/// This will also track the current field read, and the total number of fields.
/// This will automically return a default value if you try to read past the
/// maximum amount of fields.
///
/// This is the core of th forwards compatibility in this protocol.
pub struct StructReader<'a, 'b> {
  reader:        &'a mut MessageReader<'b>,
  current_field: u64,
  max_fields:    u64,
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

  /// Moves the reader back 1 byte. This is used when we read a header, then
  /// need to read it again. This helps make sure the buffer is always in a
  /// valid state.
  ///
  /// # Panics
  /// - If the buffer at index 0.
  fn undo_read_byte(&mut self) {
    self.idx = self.idx.checked_sub(1).expect("cannot move buffr back 1 (at index 0)");
  }

  /// Reads a 3 bit header for a new field. The `u8` returned is the remaining
  /// bits, shifted right by 3. So this `u8` will only have 5 bits of data set.
  ///
  /// This is private, as the caller can break the state of this reader if they
  /// do not handle the result correctly.
  fn read_header(&mut self) -> Result<(Header, u8)> {
    let val = self.read_byte()?;
    Ok((Header::from_id(val & 0x07).ok_or(ReadError::InvalidHeader(val & 0x07))?, val >> 3))
  }

  /// Reads any message. This will return an error if the buffer doesn't have
  /// enough bytes, or if the header is invalid.
  ///
  /// Avoid this if possible. If a struct is read, this will simply return a
  /// list of `Message`s, which is harder to work with. If you are expecting a
  /// certain type, [`read`](Self::read) will be much more effective.
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
  /// the 5th bit (0x10) is not set, this will not read anything.
  ///
  /// This is private, as this is doesn't read a `Header`.
  fn read_varint(&mut self, header: u8) -> Result<u64> {
    if header & 0x10 == 0 {
      return Ok(header.into());
    }

    let mut out = header as u64;
    let mut i = 0;
    let mut v;
    loop {
      v = self.read_byte()?;
      let done = v & 0x80 == 0;
      out |= ((v as u64) & !0x80) << (i * 7 + 5); // We start with 5 bits set
      if done {
        break;
      }
      i += 1;
      // (64 - 5) / 7 = 8.42, so we need 9 bytes of space
      if i >= 9 {
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
    let n = self.read_byte()? as u32
      | (self.read_byte()? as u32) << 8
      | (self.read_byte()? as u32) << 16
      | (self.read_byte()? as u32) << 24;
    Ok(f32::from_bits(n))
  }
  /// Reads a double from the buffer. This will simply read 8 bytes, and convert
  /// them into a double.
  ///
  /// This is private, as it doesn't read a `Header`.
  fn read_double(&mut self) -> Result<f64> {
    let n = self.read_byte()? as u64
      | (self.read_byte()? as u64) << 8
      | (self.read_byte()? as u64) << 16
      | (self.read_byte()? as u64) << 24
      | (self.read_byte()? as u64) << 32
      | (self.read_byte()? as u64) << 40
      | (self.read_byte()? as u64) << 48
      | (self.read_byte()? as u64) << 56;
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

  /// Reads a struct. This will return an error if the header read is not a
  /// `Struct` header, or if any of the fields of the struct are invalid.
  pub fn read_struct<S: StructRead>(&mut self) -> Result<S> {
    let (header, extra) = self.read_header()?;
    match header {
      Header::Struct => {
        let max_fields = self.read_varint(extra)?;
        S::read_struct(&mut StructReader { reader: self, current_field: 0, max_fields })
      }
      _ => {
        // We must keep the buffer at a valid state, so we undo the `read_header` call
        // above.
        self.undo_read_byte();
        let msg = self.read_any()?;
        Err(ReadError::WrongMessage(msg, header))
      }
    }
  }
  pub fn read_enum<E: EnumRead>(&mut self) -> Result<E> {
    let (variant, field) = self.read_any()?.into_enum()?;
    E::read_enum(variant, field)
  }
  /// Reads a byte array. If the header is not a `Bytes` header, this will
  /// return an error.
  pub fn read_bytes(&mut self) -> Result<Vec<u8>> { self.read_any()?.into_bytes() }
}

impl StructReader<'_, '_> {
  /// Reads a single field.
  ///
  /// # Panics
  /// - The `field` must be larger than the previous field.
  pub fn read<T: Default + MessageRead>(&mut self, mut field: u64) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    while field < self.current_field {
      self.reader.read_none()?;
      field += 1;
    }
    self.current_field = field + 1;
    if field >= self.max_fields {
      Ok(T::default())
    } else {
      T::read(self.reader)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn simple() {
    #[derive(Debug, Clone, PartialEq)]
    struct EmptyStruct {}
    impl StructRead for EmptyStruct {
      fn read_struct(_m: &mut StructReader) -> Result<Self> { Ok(EmptyStruct {}) }
    }
    #[derive(Debug, Clone, PartialEq)]
    struct IntStruct {
      a: i32,
      b: u8,
    }
    impl StructRead for IntStruct {
      fn read_struct(m: &mut StructReader) -> Result<Self> {
        Ok(IntStruct { a: m.read(0)?, b: m.read(1)? })
      }
    }
    #[derive(Debug, Clone, PartialEq)]
    enum SampleEnum {
      A,
      B,
      C,
      D,
    }
    impl EnumRead for SampleEnum {
      fn read_enum(variant: u64, field: Message) -> Result<Self> {
        Ok(match variant {
          0 => Self::A,
          1 => Self::B,
          2 => Self::C,
          3 => Self::D,
          _ => return Err(ReadError::InvalidVariant(variant)),
        })
      }
    }

    let msg = [
      // A None
      0b000,
      // A VarInt
      0b001 | 12 << 3, // A 5 bit varint can store 0-15 without needing another byte.
      // A Float
      0b010,
      0,
      0,
      0,
      0,
      // A Double
      0b011,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      // A struct with no fields
      0b100 | 0 << 3,
      // A struct with 2 int fields
      0b100 | 2 << 3,
      0b001 | super::super::zig(-3_i8) << 3,
      0b001 | 10 << 3,
      // An enum, at variant 1, with no data
      0b101 | 1 << 3,
      0b000,
      // A byte array of 5 bytes
      0b110 | 5 << 3,
      b'H',
      b'e',
      b'l',
      b'l',
      b'o',
    ];
    let mut m = MessageReader::new(&msg);
    assert_eq!(m.index(), 0);
    assert_eq!(m.read_none().unwrap(), ());
    assert_eq!(m.index(), 1);
    assert_eq!(m.read_u8().unwrap(), 12);
    assert_eq!(m.index(), 2);
    assert_eq!(m.read_f32().unwrap(), 0.0);
    assert_eq!(m.index(), 7);
    assert_eq!(m.read_f64().unwrap(), 0.0);
    assert_eq!(m.index(), 16);
    assert_eq!(m.read_struct::<EmptyStruct>().unwrap(), EmptyStruct {});
    assert_eq!(m.index(), 17);
    assert_eq!(m.read_struct::<IntStruct>().unwrap(), IntStruct { a: -3, b: 10 });
    assert_eq!(m.index(), 20);
    assert_eq!(m.read_enum::<SampleEnum>().unwrap(), SampleEnum::B);
    assert_eq!(m.index(), 22);
    assert_eq!(m.read_bytes().unwrap(), b"Hello");
    assert_eq!(m.index(), 28);
    assert!(matches!(m.read_none().unwrap_err(), ReadError::EOF));

    /*
    let mut m = MessageReader::new(&[127, 0, 0, 1]);
    assert_eq!(m.read_u16().unwrap(), 127);
    assert_eq!(m.read_u16().unwrap(), 256);
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::EOF));
    */
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

use super::{zag, Header, Message};

use std::{error::Error, fmt, string::FromUtf8Error};

type Result<T> = std::result::Result<T, ReadError>;

/// An error while reading a field. This can happen if the end of the internal
/// buffer is reached, or if a varint has too many bytes.
///
/// There are two variants here: [`Valid`](Self::Valid) and
/// [`Invalid`](Self::Invalid). These are for error recovery. If the error is a
/// [`Valid`](Self::Valid) error, then the [`MessageReader`] is in a valid
/// state, and you can continue reading. Otherwise, the state of the
/// [`MessageReader`] is undefined.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadError {
  Valid(ValidReadError),
  Invalid(InvalidReadError),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ValidReadError {
  /// This happens if we read a string, and its not valid UTF8. This is easy to
  /// recover from, as it happens after we know the length of the buffer (so we
  /// can just skip this field).
  InvalidUtf8(FromUtf8Error),
  /// Returned if an enum variant is invalid. This likely means we are reading
  /// an enum variant from a newer client, so we should just ignore this and
  /// continue reading.
  InvalidVariant(u64, Message),
  /// This happens if we try to read a specific field, and get a different type.
  /// Everything was valid on the wire, so this is recoverable.
  WrongMessage(Message, Header),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum InvalidReadError {
  /// This happens when reading a buffer (byte array or string) and the length
  /// prefix extends beyond the internal data. This is similar to EOF, and is
  /// unrecoverable.
  InvalidBufLength,
  /// Happens if a varint is too long. This likely means the data was corrupted,
  /// and we cannot recover.
  VarIntTooLong,
  /// This happens if the 3 bit header is invalid. This either means we are
  /// talking to a newer version of the protocol, or the data is corrupted.
  /// Either way, we cannot recover.
  InvalidHeader(u8),
  /// This happens if we try to read something and there are no bytes left.
  EOF,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::Valid(e) => write!(f, "read error (buffer is still valid): {e}"),
      Self::Invalid(e) => write!(f, "read error (buffer is now invalid): {e}"),
    }
  }
}
impl fmt::Display for ValidReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::InvalidUtf8(e) => write!(f, "invalid utf8: {}", e),
      Self::InvalidVariant(variant, msg) => {
        write!(f, "invalid variant: {variant} (data: {msg:?})")
      }
      Self::WrongMessage(m, header) => {
        write!(f, "got message {m:?}, expected message {header:?}")
      }
    }
  }
}
impl fmt::Display for InvalidReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::VarIntTooLong => write!(f, "failed to read field: varint was too long"),
      Self::InvalidBufLength => write!(f, "failed to read field: buffer was too long"),
      Self::InvalidHeader(header) => {
        write!(f, "failed to read field: invalid header {header:#x}")
      }
      Self::EOF => write!(f, "failed to read field: eof reached"),
    }
  }
}

impl From<ValidReadError> for ReadError {
  fn from(e: ValidReadError) -> Self { ReadError::Valid(e) }
}
impl From<InvalidReadError> for ReadError {
  fn from(e: InvalidReadError) -> Self { ReadError::Invalid(e) }
}
impl From<FromUtf8Error> for ReadError {
  fn from(e: FromUtf8Error) -> Self { ReadError::Valid(e.into()) }
}
impl From<FromUtf8Error> for ValidReadError {
  fn from(e: FromUtf8Error) -> Self { ValidReadError::InvalidUtf8(e.into()) }
}

impl Error for ReadError {}
impl Error for ValidReadError {}
impl Error for InvalidReadError {}

// TODO:
// Types:
// 0x00 => byte (signed or unsigned)
// 0x01 => int (signed or unsigned), varint encoded
// 0x02 => float
// 0x03 => double
// 0x04 => struct
// 0x05 => enum

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
    Ok((Header::from_id(val & 0x07).ok_or(InvalidReadError::InvalidHeader(val & 0x07))?, val >> 3))
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
      Err(InvalidReadError::EOF.into())
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
        return Err(InvalidReadError::VarIntTooLong.into());
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
      Err(InvalidReadError::InvalidBufLength.into())
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
    /// - If there are no remaining bytes, a [`InvalidReadError::EOF`] is returned.
    /// - If the varint parsed is too large, then a
    ///   [`InvalidReadError::VarIntTooLong`] is returned.
    /// - If the header read is not a `VarInt`, a [`ValidReadError::WrongMessage`]
    ///   is returned.
    pub fn $reader(&mut self) -> Result<$ret> {
      self.read_any()?.into_varint()?.try_into().map_err(|_| InvalidReadError::VarIntTooLong.into())
    }
  };
}
macro_rules! read_signed {
  ( $reader:ident, $ret:ty ) => {
    /// Reads a field, and makes sure that it is an 8 bit integer.
    ///
    /// Errors:
    /// - If there are no remaining bytes, a [`InvalidReadError::EOF`] is returned.
    /// - If the varint parsed is too large, then a
    ///   [`InvalidReadError::VarIntTooLong`] is returned.
    /// - If the header read is not a `VarInt`, a [`ValidReadError::WrongMessage`]
    ///   is returned.
    pub fn $reader(&mut self) -> Result<$ret> {
      self
        .read_any()?
        .into_varint()?
        .try_into()
        .map_err(|_| InvalidReadError::VarIntTooLong.into())
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
      Err(InvalidReadError::VarIntTooLong.into())
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
        Err(ValidReadError::WrongMessage(msg, header).into())
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
  pub fn read<T: Default + MessageRead>(&mut self, field: u64) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      match self.reader.read_none() {
        Ok(_) | Err(ReadError::Valid(_)) => {}
        Err(ReadError::Invalid(e)) => return Err(e.into()),
      }
      if self.current_field >= self.max_fields {
        return Ok(T::default());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Ok(T::default())
    } else {
      match T::read(self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Ok(T::default()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
  struct RemovedFieldStruct {
    a: u8,
    b: u8,
  }
  impl StructRead for RemovedFieldStruct {
    fn read_struct(m: &mut StructReader) -> Result<Self> {
      Ok(RemovedFieldStruct { a: m.read(0)?, b: m.read(2)? })
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
        _ => return Err(ValidReadError::InvalidVariant(variant, field).into()),
      })
    }
  }

  #[test]
  fn simple() {
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
    assert!(matches!(m.read_none().unwrap_err(), ReadError::Invalid(InvalidReadError::EOF)));

    /*
    let mut m = MessageReader::new(&[127, 0, 0, 1]);
    assert_eq!(m.read_u16().unwrap(), 127);
    assert_eq!(m.read_u16().unwrap(), 256);
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::EOF));
    */
  }

  #[test]
  fn missing_fields() {
    let msg = [
      // A struct with no fields
      0b100 | 0 << 3,
      // A struct with 1 field (a), set to some valid number
      0b100 | 1 << 3,
      0b001 | super::super::zig(-2_i8) << 3,
      // A struct with 1 field (a), set to some invalid field
      0b100 | 1 << 3,
      0b000, // none
      // A struct with 1 field (a), set to some invalid field
      0b100 | 1 << 3,
      0b110, // an empty byte array
      // A struct with 2 fields, with 1 set to some invalid field
      0b100 | 2 << 3,
      0b000,          // none
      0b001 | 3 << 3, // an int
      // A struct with 2 fields (both valid), but being read by a struct with 3 fields
      0b100 | 2 << 3,
      0b001 | 2 << 3, // an int
      0b001 | 3 << 3, // an int
    ];
    let mut m = MessageReader::new(&msg);
    assert_eq!(m.read_struct::<IntStruct>().unwrap(), IntStruct { a: 0, b: 0 });
    assert_eq!(m.read_struct::<IntStruct>().unwrap(), IntStruct { a: -2, b: 0 });
    assert_eq!(m.read_struct::<IntStruct>().unwrap(), IntStruct { a: 0, b: 0 });
    assert_eq!(m.read_struct::<IntStruct>().unwrap(), IntStruct { a: 0, b: 0 });
    assert_eq!(m.read_struct::<IntStruct>().unwrap(), IntStruct { a: 0, b: 3 });
    assert_eq!(m.read_struct::<RemovedFieldStruct>().unwrap(), RemovedFieldStruct { a: 2, b: 0 });
    assert!(matches!(
      m.read_struct::<IntStruct>().unwrap_err(),
      ReadError::Invalid(InvalidReadError::EOF)
    ));
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
    assert!(matches!(
      m.read_u32().unwrap_err(),
      ReadError::Invalid(InvalidReadError::VarIntTooLong)
    ));
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::Invalid(InvalidReadError::EOF)));
  }

  #[test]
  fn bytes() {
    let mut m = MessageReader::new(b"hello");
    assert_eq!(m.index(), 0);
    assert_eq!(&m.read_bytes().unwrap(), b"hello");
    assert_eq!(m.index(), 5);
  }
}

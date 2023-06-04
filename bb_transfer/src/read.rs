use super::{zag, Header};

use std::{error::Error, fmt, marker::PhantomData, str::Utf8Error};

type Result<T> = std::result::Result<T, ReadError>;
type InvalidResult<T> = std::result::Result<T, InvalidReadError>;

/// An error while reading a field. This can happen if the end of the internal
/// buffer is reached, or if a varint has too many bytes.
///
/// There are two variants here: [`Valid`](Self::Valid) and
/// [`Invalid`](Self::Invalid). These are for error recovery. If the error is a
/// [`Valid`](Self::Valid) error, then the [`MessageReader`] is in a valid
/// state, and you can continue reading. Otherwise, the state of the
/// [`MessageReader`] is undefined.
#[derive(Debug)]
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
  InvalidUtf8(Utf8Error),
  /// Returned if an enum variant is invalid. This likely means we are reading
  /// an enum variant from a newer client, so we should just ignore this and
  /// continue reading.
  InvalidVariant(u64),
  /// This happens if we try to read a specific field, and get a different type.
  /// Everything was valid on the wire, so this is recoverable.
  ///
  /// The first value is the header of the message received, and the second is
  /// what was expected.
  WrongMessage(Header, Header),
  /// Happens if we are missing a field for a struct. This is only ever produced
  /// when calling [`must_read`](StructReader::must_read). This function should
  /// be avoided, as it doesn't allow for any forwards compatibility.
  MissingField(u64),
  /// We read a `NonZero*`, and got zero.
  InvalidNonZero,
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
      Self::InvalidUtf8(e) => write!(f, "invalid utf8: {e}"),
      Self::InvalidVariant(variant) => {
        write!(f, "invalid variant: {variant}")
      }
      Self::WrongMessage(m, header) => {
        write!(f, "got message {m:?}, expected message {header:?}")
      }
      Self::MissingField(field) => {
        write!(f, "missing struct field {field}")
      }
      Self::InvalidNonZero => {
        write!(f, "while reading a nonzero number, got zero")
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
impl fmt::Display for MessageReader<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    writeln!(f, "Message ({} bytes) {{", self.data.len())?;
    let mut reader = MessageReader::new(self.data);
    while reader.can_read() {
      writeln!(f, "Field: {reader:#?}")?;
      reader.skip_field().unwrap();
    }
    write!(f, "}}")?;
    Ok(())
  }
}
impl fmt::Debug for MessageReader<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.clone().write_fmt(f) {
      Ok(()) => Ok(()),
      Err(e) => writeln!(f, "  Err({e}),"),
    }
  }
}

impl MessageReader<'_> {
  fn write_fmt(&mut self, f: &mut fmt::Formatter) -> Result<()> {
    let (header, extra) = self.read_header()?;
    match header {
      Header::None => write!(f, "None").unwrap(),
      Header::VarInt => {
        let v = self.read_varint(extra)?;
        write!(f, "VarInt({v} {v:#x})").unwrap();
      }
      Header::Float => {
        let v = self.read_float()?;
        write!(f, "Float({v})").unwrap();
      }
      Header::Double => {
        let v = self.read_double()?;
        write!(f, "Double({v})").unwrap();
      }
      Header::Struct => {
        let num_fields = self.read_varint(extra)?;
        let mut tup = f.debug_tuple("Struct");
        for _ in 0..num_fields {
          tup.field(&self);
          self.skip_field()?;
        }
        tup.finish().unwrap();
      }
      Header::Enum => {
        let variant = self.read_varint(extra)?;
        let mut tup = f.debug_tuple("Enum");
        tup.field(&variant);
        tup.field(self);
        self.skip_field()?;
        tup.finish().unwrap();
      }
      Header::Bytes => {
        let len = self.read_varint(extra)? as usize;
        let data = self.read_buf(len)?;
        if data.is_empty() {
          write!(f, "Bytes(len: 0) {{}}").unwrap();
        } else if data.len() < 32 {
          let mut s = f.debug_struct("Bytes");
          s.field("data", &data);
          match std::str::from_utf8(data) {
            Ok(v) => s.field("str", &v),
            Err(e) => s.field("str", &e),
          };
          s.finish().unwrap();
        } else {
          write!(f, "Bytes(len: {})", data.len()).unwrap();
        }
      }
      Header::List => {
        let len = self.read_varint(extra)?;
        let mut list = f.debug_list();
        for _ in 0..len {
          list.entry(&self);
          self.skip_field()?;
        }
        list.finish().unwrap();
      }
    }
    Ok(())
  }
}

impl From<ValidReadError> for ReadError {
  fn from(e: ValidReadError) -> Self { ReadError::Valid(e) }
}
impl From<InvalidReadError> for ReadError {
  fn from(e: InvalidReadError) -> Self { ReadError::Invalid(e) }
}
impl From<Utf8Error> for ReadError {
  fn from(e: Utf8Error) -> Self { ReadError::Valid(e.into()) }
}
impl From<Utf8Error> for ValidReadError {
  fn from(e: Utf8Error) -> Self { ValidReadError::InvalidUtf8(e) }
}

impl Error for ReadError {}
impl Error for ValidReadError {}
impl Error for InvalidReadError {}

/// A trait for anything that can be read from a [`MessageReader`].
pub trait MessageRead<'a> {
  /// Reads a value of Self from the reader.
  fn read(reader: &mut MessageReader<'a>) -> Result<Self>
  where
    Self: Sized;
}
/// A trait for any struct that can be read from a [`MessageReader`].
pub trait StructRead<'a> {
  /// Reads a value of Self from the given struct fields.
  fn read_struct(reader: StructReader<'a>) -> Result<Self>
  where
    Self: Sized;
}
/// A trait for any enum that can be read from a [`MessageReader`].
pub trait EnumRead<'a> {
  /// Reads a value of Self from the given variant and message.
  fn read_enum(reader: EnumReader<'a>) -> Result<Self>
  where
    Self: Sized;
}

/// Wrapper around a byte array for reading fields. Every function on this type
/// will return the same value that was written in the
/// [`MessageWrite`](super::MessageWrite).
///
/// See the [crate] level docs for how fields are decoded.
#[derive(Clone)]
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
/// This will automatically return a default value if you try to read past the
/// maximum amount of fields.
///
/// This is the core of th forwards compatibility in this protocol.
#[derive(Debug)]
pub struct StructReader<'a> {
  reader:        MessageReader<'a>,
  current_field: u64,
  max_fields:    u64,
}

/// Wrapper around a partially parsed enum. This is the enum equivalent of
/// [`StructReader`].
#[derive(Debug)]
pub struct EnumReader<'a> {
  reader:        MessageReader<'a>,
  variant:       u64,
  current_field: u64,
  max_fields:    u64,
}

/// Wrapper around a list.
#[derive(Debug)]
pub struct ListReader<'a, T> {
  reader:  MessageReader<'a>,
  current: u64,
  len:     u64,
  phantom: PhantomData<T>,
}

impl<'a> MessageReader<'a> {
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
    T: MessageRead<'a>,
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
  fn read_header(&mut self) -> InvalidResult<(Header, u8)> {
    let val = self.read_byte()?;
    Ok((Header::from_id(val & 0x07).ok_or(InvalidReadError::InvalidHeader(val & 0x07))?, val >> 3))
  }

  /// Advances past the given number of fields.
  pub fn skip_fields(&mut self, fields: u64) -> InvalidResult<()> {
    for _ in 0..fields {
      self.skip_field()?;
    }
    Ok(())
  }

  /// Skips a single field.
  pub fn skip_field(&mut self) -> InvalidResult<()> {
    let (header, extra) = self.read_header()?;
    match header {
      Header::None => {}
      Header::VarInt => {
        self.read_varint(extra)?;
      }
      Header::Float => {
        self.read_float()?;
      }
      Header::Double => {
        self.read_double()?;
      }
      Header::Struct => {
        let num_fields = self.read_varint(extra)?;
        self.skip_fields(num_fields)?;
      }
      Header::Enum => {
        let _variant = self.read_varint(extra)?;
        self.skip_field()?;
      }
      Header::Bytes => {
        let len = self.read_varint(extra)? as usize;
        self.skip_bytes(len)?;
      }
      Header::List => {
        let len = self.read_varint(extra)?;
        self.skip_fields(len)?;
      }
    }
    Ok(())
  }

  /// Reads a single byte from the buffer. Returns an error if the reader has
  /// read the entire buffer.
  ///
  /// This is private, as this is doesn't read a `Header`.
  fn read_byte(&mut self) -> InvalidResult<u8> {
    if self.idx >= self.data.len() {
      Err(InvalidReadError::EOF)
    } else {
      self.idx += 1;
      Ok(self.data[self.idx - 1])
    }
  }
  /// Reads a varint from the buffer. The given value is a 5 bit LSB header. If
  /// the 5th bit (0x10) is not set, this will not read anything.
  ///
  /// This is private, as this is doesn't read a `Header`.
  fn read_varint(&mut self, header: u8) -> InvalidResult<u64> {
    if header & 0x10 == 0 {
      return Ok(header.into());
    }

    let mut out = header as u64 & 0x0f; // We only want the 4 LSB
    let mut i = 0;
    let mut v;
    loop {
      v = self.read_byte()?;
      let done = v & 0x80 == 0;
      out |= ((v as u64) & !0x80) << (i * 7 + 4); // We start with a 5 bit number, so 4 bits are set
      if done {
        break;
      }
      i += 1;
      // (64 - 5) / 7 = 8.42, so we need 9 bytes of space
      if i >= 9 {
        return Err(InvalidReadError::VarIntTooLong);
      }
    }
    Ok(out)
  }
  /// Reads a float from the buffer. This will simply read 4 bytes, and convert
  /// them into a float.
  ///
  /// This is private, as it doesn't read a `Header`.
  fn read_float(&mut self) -> InvalidResult<f32> {
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
  fn read_double(&mut self) -> InvalidResult<f64> {
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
  fn read_buf(&mut self, len: usize) -> InvalidResult<&'a [u8]> {
    if self.idx + len > self.data.len() {
      Err(InvalidReadError::InvalidBufLength)
    } else {
      let out = &self.data[self.idx..self.idx + len];
      self.idx += len;
      Ok(out)
    }
  }

  /// Skips the given number of bytes.
  fn skip_bytes(&mut self, len: usize) -> InvalidResult<()> {
    if self.idx + len > self.data.len() {
      Err(InvalidReadError::InvalidBufLength)
    } else {
      self.idx += len;
      Ok(())
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
      self.read_u64()?.try_into().map_err(|_| InvalidReadError::VarIntTooLong.into())
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
        .read_u64()?
        .try_into()
        .map_err(|_| InvalidReadError::VarIntTooLong.into())
        .map(|v| zag(v))
    }
  };
}

impl<'a> MessageReader<'a> {
  /// Reads a single field. If this is not a `None` field, this returns a
  /// [`ValidReadError::WrongMessage`] error.
  pub fn read_none(&mut self) -> Result<()> {
    let (header, _) = self.read_header()?;
    if header != Header::None {
      Err(ValidReadError::WrongMessage(header, Header::None).into())
    } else {
      Ok(())
    }
  }

  /// Reads a field. The field must be a `VarInt`, and the value must not be
  /// larger than 1. This field (including the header) will always use 1 byte.
  pub fn read_bool(&mut self) -> Result<bool> {
    let num = self.read_u64()?;
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
  /// Reads a `u64` from the internal buffer. This will read a header, and
  /// return a [`ValidReadError::WrongMessage`] error if it is another type.
  /// This will then read the remaining bytes of the varint.
  pub fn read_u64(&mut self) -> Result<u64> {
    let (header, extra) = self.read_header()?;
    if header != Header::VarInt {
      Err(ValidReadError::WrongMessage(header, Header::VarInt).into())
    } else {
      self.read_varint(extra).map_err(Into::into)
    }
  }

  read_signed!(read_i8, i8);
  read_signed!(read_i16, i16);
  read_signed!(read_i32, i32);
  read_signed!(read_i64, i64);

  /// Reads a float. This will return an error if the header read is not a
  /// `Float` header.
  pub fn read_f32(&mut self) -> Result<f32> {
    let (header, _) = self.read_header()?;
    if header != Header::Float {
      Err(ValidReadError::WrongMessage(header, Header::Float).into())
    } else {
      self.read_float().map_err(Into::into)
    }
  }
  /// Reads a double. This will return an error if the header read is not a
  /// `Double` header.
  pub fn read_f64(&mut self) -> Result<f64> {
    let (header, _) = self.read_header()?;
    if header != Header::Double {
      Err(ValidReadError::WrongMessage(header, Header::Double).into())
    } else {
      self.read_double().map_err(Into::into)
    }
  }

  /// Reads a struct. This will return an error if the header read is not a
  /// `Struct` header, or if any of the fields of the struct are invalid.
  pub fn read_struct<S: StructRead<'a>>(&mut self) -> Result<S> {
    let (header, extra) = self.read_header()?;
    match header {
      Header::Struct => {
        let max_fields = self.read_varint(extra)?;
        let start_idx = self.idx;
        // Advance out `self.idx` ahead to the end of this struct, before passing it to
        // `read_struct`. This ensures that we stay in a valid state, even if the
        // StructReader is dropped before reading all fields.
        self.skip_fields(max_fields)?;
        S::read_struct(StructReader {
          reader: MessageReader { data: self.data, idx: start_idx },
          current_field: 0,
          max_fields,
        })
      }
      m => {
        // We must keep the buffer at a valid state, so we undo the `read_header` call
        // above. We also want to skip this field (whatever it might be), so that the
        // next call can get the next field.
        self.undo_read_byte();
        self.skip_field()?;
        Err(ValidReadError::WrongMessage(m, Header::Struct).into())
      }
    }
  }
  /// Reads a struct. This will return an error if the header read is not a
  /// `Struct` header, or if any of the fields of the struct are invalid.
  pub fn read_struct_with<S>(
    &mut self,
    f: impl FnOnce(StructReader<'a>) -> Result<S>,
  ) -> Result<S> {
    let (header, extra) = self.read_header()?;
    match header {
      Header::Struct => {
        let max_fields = self.read_varint(extra)?;
        let start_idx = self.idx;
        // Advance out `self.idx` ahead to the end of this struct, before passing it to
        // `read_struct`. This ensures that we stay in a valid state, even if the
        // StructReader is dropped before reading all fields.
        self.skip_fields(max_fields)?;
        f(StructReader {
          reader: MessageReader { data: self.data, idx: start_idx },
          current_field: 0,
          max_fields,
        })
      }
      m => {
        // We must keep the buffer at a valid state, so we undo the `read_header` call
        // above. We also want to skip this field (whatever it might be), so that the
        // next call can get the next field.
        self.undo_read_byte();
        self.skip_field()?;
        Err(ValidReadError::WrongMessage(m, Header::Struct).into())
      }
    }
  }
  pub fn read_enum<E: EnumRead<'a>>(&mut self) -> Result<E> {
    let (header, extra) = self.read_header()?;
    match header {
      Header::Enum => {
        let variant = self.read_varint(extra)?;
        let (header, extra) = self.read_header()?;
        match header {
          Header::Struct => {
            let max_fields = self.read_varint(extra)?;
            let start_idx = self.idx;
            // Advance out `self.idx` ahead to the end of this struct, before passing it to
            // `read_struct`. This ensures that we stay in a valid state, even if the
            // StructReader is dropped before reading all fields.
            self.skip_fields(max_fields)?;
            E::read_enum(EnumReader {
              reader: MessageReader { data: self.data, idx: start_idx },
              variant,
              current_field: 0,
              max_fields,
            })
          }
          m => {
            // We must keep the buffer at a valid state, so we undo the `read_header` call
            // above.
            self.undo_read_byte();
            self.skip_field()?;
            Err(ValidReadError::WrongMessage(m, Header::Struct).into())
          }
        }
      }
      m => {
        // We must keep the buffer at a valid state, so we undo the `read_header` call
        // above.
        self.undo_read_byte();
        self.skip_field()?;
        Err(ValidReadError::WrongMessage(m, Header::Enum).into())
      }
    }
  }
  pub fn read_enum_with<T>(&mut self, f: impl FnOnce(EnumReader) -> Result<T>) -> Result<T> {
    let (header, extra) = self.read_header()?;
    match header {
      Header::Enum => {
        let variant = self.read_varint(extra)?;
        let (header, extra) = self.read_header()?;
        match header {
          Header::Struct => {
            let max_fields = self.read_varint(extra)?;
            let start_idx = self.idx;
            // Advance out `self.idx` ahead to the end of this struct, before passing it to
            // `read_struct`. This ensures that we stay in a valid state, even if the
            // StructReader is dropped before reading all fields.
            self.skip_fields(max_fields)?;
            f(EnumReader {
              reader: MessageReader { data: self.data, idx: start_idx },
              variant,
              current_field: 0,
              max_fields,
            })
          }
          m => {
            // We must keep the buffer at a valid state, so we undo the `read_header` call
            // above.
            self.undo_read_byte();
            self.skip_field()?;
            Err(ValidReadError::WrongMessage(m, Header::Struct).into())
          }
        }
      }
      m => {
        // We must keep the buffer at a valid state, so we undo the `read_header` call
        // above.
        self.undo_read_byte();
        self.skip_field()?;
        Err(ValidReadError::WrongMessage(m, Header::Enum).into())
      }
    }
  }
  /// Reads a byte array. If the header is not a `Bytes` header, this will
  /// return a [`ValidReadError::WrongMessage`] error.
  pub fn read_bytes(&mut self) -> Result<&'a [u8]> {
    let (header, extra) = self.read_header()?;
    if header != Header::Bytes {
      Err(ValidReadError::WrongMessage(header, Header::Bytes).into())
    } else {
      let len = self.read_varint(extra)?;
      self.read_buf(len as usize).map_err(Into::into)
    }
  }
  /// Reads a string. If the header is not a `Bytes` header, this will
  /// return a [`ValidReadError::WrongMessage`] error.
  pub fn read_str(&mut self) -> Result<&'a str> { Ok(std::str::from_utf8(self.read_bytes()?)?) }

  /// Reads a list of type `T`. This will return a ListReader, which is an
  /// iterator over `Result<T, ReadError>`.
  pub fn read_list<T>(&mut self) -> Result<ListReader<'a, T>> {
    let (header, extra) = self.read_header()?;
    if header != Header::List {
      Err(ValidReadError::WrongMessage(header, Header::Bytes).into())
    } else {
      let len = self.read_varint(extra)?;
      let reader = ListReader {
        reader: MessageReader { data: self.data, idx: self.idx },
        current: 0,
        len,
        phantom: PhantomData,
      };
      self.skip_fields(len)?;
      Ok(reader)
    }
  }

  /// Reads a list of type `T`. `f` will be called for every element in the
  /// list.
  pub fn read_list_with<T>(
    &mut self,
    mut f: impl FnMut(&mut MessageReader) -> Result<T>,
  ) -> Result<Vec<T>> {
    let (header, extra) = self.read_header()?;
    if header != Header::List {
      Err(ValidReadError::WrongMessage(header, Header::Bytes).into())
    } else {
      let len = self.read_varint(extra)?;
      let mut list = vec![];
      for _ in 0..len {
        list.push(f(self)?);
      }
      Ok(list)
    }
  }
}

impl<'a> StructReader<'a> {
  /// Reads a single field. Returns a default value if the field is not present.
  ///
  /// # Panics
  /// - The `field` must be larger than the previous field.
  pub fn read<T: Default + MessageRead<'a>>(&mut self, field: u64) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Ok(T::default());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Ok(T::default())
    } else {
      match T::read(&mut self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Ok(T::default()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }
  /// Reads a single field. Returns a default value if the field is not present.
  ///
  /// # Panics
  /// - The `field` must be larger than the previous field.
  pub fn read_with<T: Default>(
    &mut self,
    field: u64,
    f: impl FnOnce(&mut MessageReader) -> Result<T>,
  ) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Ok(T::default());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Ok(T::default())
    } else {
      match f(&mut self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Ok(T::default()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }
  /// Reads a single list field. Returns an empty list if not present.
  ///
  /// # Panics
  /// - The `field` must be larger than the previous field.
  pub fn read_list_with<T>(
    &mut self,
    field: u64,
    mut f: impl FnMut(&mut MessageReader) -> Result<T>,
  ) -> Result<Vec<T>> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Ok(vec![]);
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Ok(vec![])
    } else {
      Ok(self.reader.read_list_with(|r| f(r))?)
    }
  }
  /// Reads a single field. The [`read`](Self::read) function will simply return
  /// a default value if the field is not present. For forwards compatibility,
  /// that should always be preferred. However, if you cannot implement
  /// [`Default`] for a field, this function can be used instead. This will
  /// return an error if the field is not present.
  ///
  /// Only use this if you know that you will *never* remove this field in the
  /// future!
  pub fn must_read<T: MessageRead<'a>>(&mut self, field: u64) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Err(ValidReadError::MissingField(field).into());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Err(ValidReadError::MissingField(field).into())
    } else {
      match T::read(&mut self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Err(ValidReadError::MissingField(field).into()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }
}

impl<'a> EnumReader<'a> {
  /// Returns the variant of this enum reader. Should be matched against in
  /// implementers of [`EnumRead`].
  pub fn variant(&self) -> u64 { self.variant }
  /// Returns an error that should be generated when the enum variant is
  /// invalid.
  pub fn invalid_variant(&mut self) -> ReadError {
    ValidReadError::InvalidVariant(self.variant).into()
  }

  /// Reads a single field.
  ///
  /// # Panics
  /// - If `field` is less than the previous field.
  pub fn read<T: Default + MessageRead<'a>>(&mut self, field: u64) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Ok(T::default());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Ok(T::default())
    } else {
      match T::read(&mut self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Ok(T::default()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }
  /// Reads a single field, using the given reader function.
  ///
  /// # Panics
  /// - If `field` is less than the previous field.
  pub fn read_with<T: Default>(
    &mut self,
    field: u64,
    f: impl FnOnce(&mut MessageReader) -> Result<T>,
  ) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Ok(T::default());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Ok(T::default())
    } else {
      match f(&mut self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Ok(T::default()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }

  /// Reads a single field. Returns an error if it is not present.
  ///
  /// # Panics
  /// - If `field` is less than the previous field.
  pub fn must_read<T: MessageRead<'a>>(&mut self, field: u64) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Err(ValidReadError::MissingField(field).into());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Err(ValidReadError::MissingField(field).into())
    } else {
      match T::read(&mut self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Err(ValidReadError::MissingField(field).into()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }
  /// Reads a single field. Returns an error if it is not present.
  ///
  /// # Panics
  /// - If `field` is less than the previous field.
  pub fn must_read_with<T>(
    &mut self,
    field: u64,
    f: impl FnOnce(&mut MessageReader) -> Result<T>,
  ) -> Result<T> {
    if field < self.current_field {
      panic!(
        "cannot read field that is < current field: {field} (current_field: {})",
        self.current_field,
      );
    }
    self.current_field += 1;
    while self.current_field <= field {
      self.reader.skip_field()?;
      if self.current_field >= self.max_fields {
        return Err(ValidReadError::MissingField(field).into());
      }
      self.current_field += 1;
    }
    if field >= self.max_fields {
      Err(ValidReadError::MissingField(field).into())
    } else {
      match f(&mut self.reader) {
        Ok(v) => Ok(v),
        Err(ReadError::Valid(_)) => Err(ValidReadError::MissingField(field).into()),
        Err(ReadError::Invalid(e)) => Err(e.into()),
      }
    }
  }
}

impl<'a, T> Iterator for ListReader<'a, T>
where
  T: MessageRead<'a>,
{
  type Item = Result<T>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.current < self.len {
      self.current += 1;
      Some(self.reader.read())
    } else {
      None
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let curr = self.current as usize;
    let len = self.len as usize;
    (len - curr, Some(len - curr))
  }
}
impl<'a, T> ExactSizeIterator for ListReader<'a, T>
where
  T: MessageRead<'a>,
{
  fn len(&self) -> usize { self.len as usize - self.current as usize }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug, Clone, PartialEq)]
  struct EmptyStruct {}
  impl StructRead<'_> for EmptyStruct {
    fn read_struct(_m: StructReader) -> Result<Self> { Ok(EmptyStruct {}) }
  }
  #[derive(Debug, Clone, PartialEq)]
  struct IntStruct {
    a: i32,
    b: u8,
  }
  impl StructRead<'_> for IntStruct {
    fn read_struct(mut m: StructReader) -> Result<Self> {
      Ok(IntStruct { a: m.read(0)?, b: m.read(1)? })
    }
  }
  #[derive(Debug, Clone, PartialEq)]
  struct RemovedFieldStruct {
    a: u8,
    b: u8,
  }
  impl StructRead<'_> for RemovedFieldStruct {
    fn read_struct(mut m: StructReader) -> Result<Self> {
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
  impl EnumRead<'_> for SampleEnum {
    fn read_enum(mut m: EnumReader) -> Result<Self> {
      Ok(match m.variant() {
        0 => Self::A,
        1 => Self::B,
        2 => Self::C,
        3 => Self::D,
        _ => return Err(m.invalid_variant()),
      })
    }
  }
  #[derive(Debug, Clone, PartialEq)]
  enum DataEnum {
    A,
    B(i8),
    C(u8, u8),
  }
  impl EnumRead<'_> for DataEnum {
    fn read_enum(mut m: EnumReader) -> Result<Self> {
      Ok(match m.variant() {
        0 => Self::A,
        1 => Self::B(m.read(0)?),
        2 => Self::C(m.read(0)?, m.read(1)?),
        _ => return Err(m.invalid_variant()),
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
      0b100 | 0 << 3,
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
      // A struct with 2 fields (both valid), that will be read by a struct expecting 0 fields
      // (this makes sure we advance the buffer past all the fields).
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
    assert_eq!(m.read_struct::<EmptyStruct>().unwrap(), EmptyStruct {});
    let err = m.read_struct::<IntStruct>().unwrap_err();
    assert!(matches!(err, ReadError::Invalid(InvalidReadError::EOF)), "unexpected error {err:?}");
  }

  #[test]
  fn enums() {
    let msg = [
      // An enum with no data
      0b101 | 0 << 3,
      0b100 | 0 << 3,
      // An enum storing an int
      0b101 | 1 << 3,
      0b100 | 1 << 3,
      0b001 | super::super::zig(-2_i8) << 3,
    ];
    let mut m = MessageReader::new(&msg);
    assert_eq!(m.read_enum::<DataEnum>().unwrap(), DataEnum::A);
    assert_eq!(m.read_enum::<DataEnum>().unwrap(), DataEnum::B(-2));
  }

  #[test]
  fn varints() {
    let mut m = MessageReader::new(&[
      0b001 | 0 << 3,  // 0
      0b001 | 1 << 3,  // 1
      0b001 | 15 << 3, // 15
      0b001 | 16 << 3, // 16
      1,               // ..
      0b001 | 31 << 3, // 255
      15,              // ..
    ]);
    assert_eq!(m.read_u8().unwrap(), 0);
    assert_eq!(m.read_u8().unwrap(), 1);
    assert_eq!(m.read_u8().unwrap(), 15);
    assert_eq!(m.read_u8().unwrap(), 16);
    assert_eq!(m.read_u8().unwrap(), 255);
    /*
    assert!(matches!(
      m.read_u32().unwrap_err(),
      ReadError::Invalid(InvalidReadError::VarIntTooLong)
    ));
    */
    assert!(matches!(m.read_u32().unwrap_err(), ReadError::Invalid(InvalidReadError::EOF)));
  }

  #[test]
  fn bytes() {
    let mut m = MessageReader::new(&[0b110 | 5 << 3, b'h', b'e', b'l', b'l', b'o']);
    assert_eq!(m.index(), 0);
    assert_eq!(m.read_str().unwrap(), "hello");
    assert_eq!(m.index(), 6);
  }
}

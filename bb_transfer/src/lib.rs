//! This is a very simple message reading/writing system. It was designed as a
//! tool for generated code, so that a chunk of generated code could read and
//! write some data to a byte array in a very efficient manner.
//!
//! This is not intended for general use purposes. The message has no type of
//! length or type prefix, and would be terrible to debug a bad message. This is
//! useful when you have a pre-defined spec, and you know exactly what the
//! message should look like. Then, you can read/write the correct fields, using
//! as few bytes as possible.
//!
//! The problem with protobuf for me is simply the overhead. It adds a big
//! buildscript dependency, and ends up adding a bunch of bytes that I don't
//! really need (because I know the entire message spec at compile time, I don't
//! need to have a type header on all the numbers).
//!
//! # Message Format
//!
//! Each message starts with a 3 bit header. This header tells the reader how to
//! find the length of the following fields. Note that this doesn't include
//! information about signed/unsigned integers. The 3 bit header's purpose is
//! only to determine the length of the following data. See [`Header`].

// Messages are encoded using one of three methods: fixed field encoding,
// varible field encoding, and varible data encoding.
//
// Fixed field encoding is used on `bool`, `i8`, `u8`, `i16`, `u16`, `f32`, and
// `f64`. This is used when the field is a certain number of bytes, and it will
// never change size. Note that this is more of a general concept of encoding.
// Each of the read/write functions here just writes those bytes to the buffer,
// without any type of length or type prefix.
//
// Variable field encoding is used on `i32`, `u32`, `i64`, and `u64`. This
// means that the field will only use a certain number of bytes depending on
// how large it is. So a value of 0 will only use 1 byte, while a value of 500
// uses 2 bytes. This uses varint encoding internally. See below for the
// details on how varints are encoded. The signed versions (`i32` and `i64`)
// are encoded after being [zig-zag](ZigZag) encoded.
//
// Variable data encoding is used for strings and byte arrays. This will write
// the length of the data as a `u32` into the buffer, and the copy the data in
// after that.

mod generics;
mod read;
mod write;

pub use read::{
  EnumRead, EnumReader, InvalidReadError, MessageRead, MessageReader, ReadError, StructRead,
  StructReader, ValidReadError,
};
pub use write::{MessageWrite, MessageWriter, WriteError};

/// This is a 3 bit header for every field. For example, if I write a `u8`, a 3
/// bit [`VarInt`] header will be written, with the actual data following that.
/// This tells the reading side how to parse this message. This also allows for
/// forwards compatibility, where one side of a connection updates their spec.
///
/// [`VarInt`] is also used to encode all integers, including bytes. This is so
/// that all small number only take up one byte (including the header).
///
/// [`VarInt`] doesn't specify the sign of the number. This is because we
/// read/write everyting as an unsigned number, and then use zigzag
/// encoding/decoding after the fact. It is up to the reader to determine if it
/// is expecting a signed or unsigned number.
///
/// [`VarInt`]: Header::VarInt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Header {
  /// The 5 bits following this header contain no data. This is a placeholder
  /// for things like missing struct fields, or enum variants with no data.
  None,
  /// The next 5 bits are used as the LSB of a varint. If the highest bit is
  /// set, then the next byte will also be used to store the byte. This
  /// helps a lot with small numbers, whose value usually only needs a few bits.
  VarInt,
  /// The next 5 bits are not used. The next 4 bytes are read as a 32-bit float.
  Float,
  /// The next 5 bits are not used. The next 8 bytes are read as a 64-bit
  /// double.
  Double,
  /// The next 5 bits (and following bytes) are used in the same way as
  /// `VarInt`, to specify the number of fields that follows. A message is then
  /// parsed for every following field. If a field is removed, a `None` header
  /// will be inserted to signify that there is a missing field.
  ///
  /// This is essentially a `VarInt` which specifies the length for an array of
  /// fields.
  Struct,
  /// The next 5 bits (and following bytes) are used in the same was as
  /// `VarInt`, to specify which enum variant follows. After this, a single
  /// field is read, which will specify what data this enum variant stores.
  ///
  /// The `VarInt` that this reads is less useful for a reader who doesn't know
  /// the original message format. However, it makes a reader that is expecting
  /// a certain format much more correct.
  Enum,
  /// Similar to `Struct`. A `VarInt` is read using the next 5 bits (and
  /// following bytes). This number is the number of bytes that follows.
  Bytes,
  /// Very similar to struct. The only difference is how this is read; a struct
  /// is meant to be read with an expected spec already known, only skipping
  /// unknown fields. A list on the other hand should not have a known length.
  /// So, when reading a list, the reader is given the list's length at the
  /// start.
  List,
}

impl Header {
  /// Returns the Header for this 3 bit ID. `None` will be returned for any
  /// value that outside `0..6`.
  pub fn from_id(id: u8) -> Option<Header> {
    Some(match id {
      0x00 => Self::None,
      0x01 => Self::VarInt,
      0x02 => Self::Float,
      0x03 => Self::Double,
      0x04 => Self::Struct,
      0x05 => Self::Enum,
      0x06 => Self::Bytes,
      0x07 => Self::List,
      _ => return None,
    })
  }

  /// Returns the 3 bit ID for this header. This is used when writing a message.
  pub fn id(&self) -> u8 {
    match self {
      Self::None => 0x00,
      Self::VarInt => 0x01,
      Self::Float => 0x02,
      Self::Double => 0x03,
      Self::Struct => 0x04,
      Self::Enum => 0x05,
      Self::Bytes => 0x06,
      Self::List => 0x07,
    }
  }
}

/// Encodes the number using zig zag encoding. See the [trait](ZigZag) docs
/// for more.
#[inline(always)]
pub fn zig<Z>(num: Z) -> Z::Unsigned
where
  Z: ZigZag,
{
  ZigZag::zig(num)
}

/// Decodes the number using zig zag encoding. See the [trait](ZigZag) docs
/// for more.
#[inline(always)]
pub fn zag<Z>(num: Z::Unsigned) -> Z
where
  Z: ZigZag,
{
  ZigZag::zag(num)
}

/// This is a trait for encoding and decoding negative numbers in an efficient
/// way for varint encoding.
///
/// Note: I understand that the naming of functions here is unclear. However, it
/// is funny, and will only be used internally, so I'm OK with it.
///
/// Zig-zag encoding works like so:
///
/// Original | Encoded
/// ---------|--------
/// 0        | 0
/// -1       | 1
/// 1        | 2
/// -2       | 3
///
/// As you can see, the small positive and negative values will result in small
/// unsigned values, and will therefore use less bytes in the buffer.
///
/// Specifically, the original value is doubled, and then 1 is subtracted if the
/// value is negative. This results in an encoding function (`zig`) like so: `(n
/// << 1) ^ (n >> 31)`, where >> 31 will result in all 1s if the value was
/// negative, and result in 0 if it was positive.
///
/// The decoding function (`zag`) works like so: `(n >> 1) ^ -(n & 1)`. `n & 1`
/// results in a 1 if the value was originally negative, and a 0 if not. This is
/// then negated, so that the number will be all 1s if the original was
/// negative. We then xor that with the original value, which gives us a
/// twos-complement encoded number (our original value).
pub trait ZigZag {
  /// The unsigned version of this number (`u32` if Self is `i32`, etc).
  type Unsigned;

  /// Encodes the number using zig zag encoding. See the [trait](ZigZag) docs
  /// for more.
  fn zig(n: Self) -> Self::Unsigned;
  /// Decodes the number using zig zag encoding. See the [trait](ZigZag) docs
  /// for more.
  fn zag(n: Self::Unsigned) -> Self;
}

impl ZigZag for i8 {
  type Unsigned = u8;

  #[inline(always)]
  fn zig(n: i8) -> u8 { ((n << 1) ^ (n >> 7)) as u8 }
  #[inline(always)]
  fn zag(n: u8) -> i8 { (n >> 1) as i8 ^ -((n & 1) as i8) }
}

impl ZigZag for i16 {
  type Unsigned = u16;

  #[inline(always)]
  fn zig(n: i16) -> u16 { ((n << 1) ^ (n >> 15)) as u16 }
  #[inline(always)]
  fn zag(n: u16) -> i16 { (n >> 1) as i16 ^ -((n & 1) as i16) }
}

impl ZigZag for i32 {
  type Unsigned = u32;

  #[inline(always)]
  fn zig(n: i32) -> u32 { ((n << 1) ^ (n >> 31)) as u32 }
  #[inline(always)]
  fn zag(n: u32) -> i32 { (n >> 1) as i32 ^ -((n & 1) as i32) }
}

impl ZigZag for i64 {
  type Unsigned = u64;

  #[inline(always)]
  fn zig(n: i64) -> u64 { ((n << 1) ^ (n >> 63)) as u64 }
  #[inline(always)]
  fn zag(n: u64) -> i64 { (n >> 1) as i64 ^ -((n & 1) as i64) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn zigzag() {
    assert_eq!(zig::<i32>(0), 0);
    assert_eq!(zig::<i32>(-1), 1);
    assert_eq!(zig::<i32>(1), 2);
    assert_eq!(zig::<i32>(-2), 3);
    assert_eq!(zag::<i32>(0), 0);
    assert_eq!(zag::<i32>(1), -1);
    assert_eq!(zag::<i32>(2), 1);
    assert_eq!(zag::<i32>(3), -2);
    for i in -1000..1000 {
      assert_eq!(i, zag(zig(i)));
    }
  }

  #[test]
  fn floats() {
    let mut buf = [0; 5];
    let mut m = MessageWriter::new(buf.as_mut_slice());
    m.write_f32(3.456).unwrap();
    let mut m = MessageReader::new(&buf);
    assert_eq!(m.read_f32().unwrap(), 3.456);

    let mut buf = [0; 9];
    let mut m = MessageWriter::new(buf.as_mut_slice());
    m.write_f64(3.456).unwrap();
    let mut m = MessageReader::new(&buf);
    assert_eq!(m.read_f64().unwrap(), 3.456);
  }

  #[test]
  fn read_write() {
    let mut buf = [0; 5];
    let mut m = MessageWriter::new(buf.as_mut_slice());
    m.write_u32(123525).unwrap();
    let mut m = MessageReader::new(&buf);
    assert_eq!(m.read_u32().unwrap(), 123525);
  }
}

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
//! Messages are encoded using one of three methods: fixed field encoding,
//! varible field encoding, and varible data encoding.
//!
//! Fixed field encoding is used on `bool`, `i8`, `u8`, `i16`, `u16`, `f32`, and
//! `f64`. This is used when the field is a certain number of bytes, and it will
//! never change size. Note that this is more of a general concept of encoding.
//! Each of the read/write functions here just writes those bytes to the buffer,
//! without any type of length or type prefix.
//!
//! Variable field encoding is used on `i32`, `u32`, `i64`, and `u64`. This
//! means that the field will only use a certain number of bytes depending on
//! how large it is. So a value of 0 will only use 1 byte, while a value of 500
//! uses 2 bytes. This uses varint encoding internally. See below for the
//! details on how varints are encoded. The signed versions (`i32` and `i64`)
//! are encoded after being [zig-zag](ZigZag) encoded.
//!
//! Variable data encoding is used for strings and byte arrays. This will write
//! the length of the data as a `u32` into the buffer, and the copy the data in
//! after that.

mod read;
mod write;

pub use read::{MessageRead, ReadError};
pub use write::{MessageWrite, WriteError};

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

impl ZigZag for i32 {
  type Unsigned = u32;

  #[inline(always)]
  fn zig(n: i32) -> u32 {
    ((n << 1) ^ (n >> 31)) as u32
  }
  #[inline(always)]
  fn zag(n: u32) -> i32 {
    (n >> 1) as i32 ^ -((n & 1) as i32)
  }
}

impl ZigZag for i64 {
  type Unsigned = u64;

  #[inline(always)]
  fn zig(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
  }
  #[inline(always)]
  fn zag(n: u64) -> i64 {
    (n >> 1) as i64 ^ -((n & 1) as i64)
  }
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
    let mut buf = [0; 4];
    let mut m = MessageWrite::new(&mut buf);
    m.write_f32(3.456).unwrap();
    let mut m = MessageRead::new(&buf);
    assert_eq!(m.read_f32().unwrap(), 3.456);

    let mut buf = [0; 8];
    let mut m = MessageWrite::new(&mut buf);
    m.write_f64(3.456).unwrap();
    let mut m = MessageRead::new(&buf);
    assert_eq!(m.read_f64().unwrap(), 3.456);
  }
}

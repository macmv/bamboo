use crate::util::{Buffer, BufferError};
use flate2::read::{GzDecoder, ZlibDecoder};
use std::{collections::HashMap, error::Error, fmt, io, io::Read, string::FromUtf8Error};

use super::{ParseError, Tag, NBT};

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidType(ty) => write!(f, "invalid tag type: {ty}"),
      Self::InvalidString(e) => write!(f, "invalid string: {e}"),
      Self::IO(e) => write!(f, "io error: {e}"),
      Self::BufferError(e) => write!(f, "buffer error: {e}"),
    }
  }
}

impl From<FromUtf8Error> for ParseError {
  fn from(e: FromUtf8Error) -> ParseError { ParseError::InvalidString(e) }
}
impl From<io::Error> for ParseError {
  fn from(e: io::Error) -> ParseError { ParseError::IO(e) }
}
impl From<BufferError> for ParseError {
  fn from(e: BufferError) -> ParseError { ParseError::BufferError(e) }
}

impl Error for ParseError {}

impl NBT {
  pub fn deserialize_file(buf: Vec<u8>) -> Result<Self, ParseError> {
    if buf.len() >= 2 && buf[0] == 0x1f && buf[1] == 0x8b {
      // This means its gzipped
      let mut d: GzDecoder<&[u8]> = GzDecoder::new(buf.as_ref());
      let mut buf = vec![];
      d.read_to_end(&mut buf)?;
      Self::deserialize(buf)
    } else {
      // It could be zlib compressed or not compressed
      let mut d: ZlibDecoder<&[u8]> = ZlibDecoder::new(buf.as_ref());
      let mut decompressed = vec![];
      match d.read_to_end(&mut decompressed) {
        Ok(_) => Self::deserialize(decompressed),
        Err(_) => Self::deserialize(buf),
      }
    }
  }
  /// Deserializes the given byte array as nbt data.
  pub fn deserialize(mut buf: Vec<u8>) -> Result<Self, ParseError> {
    Self::deserialize_buf(&mut Buffer::new(&mut buf))
  }
  /// Deserializes the given buffer as nbt data. This will continue reading
  /// where this buffer is currently placed, and will advance the reader to be
  /// right after the nbt data. If this function returns an error, then the
  /// buffer will be in an undefined state (it will still be safe, but there are
  /// no guarantees as too how far ahead the buffer will have been advanced).
  pub fn deserialize_buf<T: AsRef<[u8]>>(buf: &mut Buffer<T>) -> Result<Self, ParseError> {
    let ty = buf.read_u8()?;
    if ty == 0 {
      Ok(NBT::empty())
    } else {
      let len = buf.read_u16()?;
      let name = String::from_utf8(buf.read_buf(len as usize)?)?;
      Ok(NBT::new(&name, Tag::deserialize(ty, buf)?))
    }
  }
}

impl Tag {
  fn deserialize<T: AsRef<[u8]>>(ty: u8, buf: &mut Buffer<T>) -> Result<Self, ParseError> {
    match ty {
      0 => Ok(Self::End),
      1 => Ok(Self::Byte(buf.read_i8()?)),
      2 => Ok(Self::Short(buf.read_i16()?)),
      3 => Ok(Self::Int(buf.read_i32()?)),
      4 => Ok(Self::Long(buf.read_i64()?)),
      5 => Ok(Self::Float(buf.read_f32()?)),
      6 => Ok(Self::Double(buf.read_f64()?)),
      7 => {
        let len = buf.read_i32()?;
        Ok(Self::ByteArr(buf.read_buf(len as usize)?))
      }
      8 => {
        let len = buf.read_u16()?;
        match String::from_utf8(buf.read_buf(len as usize)?) {
          Ok(v) => Ok(Self::String(v)),
          Err(e) => Err(ParseError::InvalidString(e)),
        }
      }
      9 => {
        let inner_ty = buf.read_u8()?;
        let len = buf.read_i32()?;
        let mut inner = Vec::with_capacity(len as usize);
        for _ in 0..len {
          inner.push(Tag::deserialize(inner_ty, buf)?);
        }
        Ok(Self::List(inner))
      }
      10 => {
        let mut inner = HashMap::new();
        loop {
          let ty = buf.read_u8()?;
          if ty == Self::End.ty() {
            break;
          }
          let len = buf.read_u16()?;
          let name = String::from_utf8(buf.read_buf(len as usize)?).unwrap();
          let tag = Tag::deserialize(ty, buf)?;
          inner.insert(name, tag);
        }
        Ok(inner.into())
      }
      11 => {
        let len = buf.read_i32()?;
        let mut inner = Vec::with_capacity(len as usize);
        for _ in 0..len {
          inner.push(buf.read_i32()?);
        }
        Ok(Self::IntArray(inner))
      }
      12 => {
        let len = buf.read_i32()?;
        let mut inner = Vec::with_capacity(len as usize);
        for _ in 0..len {
          inner.push(buf.read_i64()?);
        }
        Ok(Self::LongArray(inner))
      }
      _ => Err(ParseError::InvalidType(ty)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deserialize() -> Result<(), ParseError> {
    let v = NBT::new(
      "hello",
      Tag::new_compound(&[
        ("small", Tag::Byte(5)),
        ("i is short", Tag::Short(7)),
        ("int time", Tag::Int(12)),
        ("mmmm long", Tag::Long(123564536)),
        ("funny number", Tag::Float(123.0)),
        ("big number", Tag::Double(123.0)),
        ("arrrrrrrr", Tag::ByteArr(vec![0, 4, 5, 7, 7, 7, 8, 9])),
        ("big str", Tag::String("hello i am a string".into())),
        (
          "str list time",
          Tag::List(vec![
            Tag::String("list elem 1".into()),
            Tag::String("list elem 2".into()),
            Tag::String("list elem 3".into()),
          ]),
        ),
        (
          "nested compound",
          Tag::new_compound(&[
            ("inner 1", Tag::new_compound(&[("num", Tag::Int(5))])),
            ("inner 2", Tag::new_compound(&[("str", Tag::String("words".into()))])),
            ("compound more", Tag::Long(12313)),
          ]),
        ),
      ]),
    );
    let new = NBT::deserialize(v.serialize())?;
    assert_eq!(new, v);

    let _expected = NBT::new(
      "Level",
      Tag::new_compound(&[
        (
          "nested compound test",
          Tag::new_compound(&[
            (
              "egg",
              Tag::new_compound(&[("name", Tag::String("Eggbert".into())), ("value", Tag::Float(0.5))]),
            ),
            (
              "ham",
              Tag::new_compound(&[("name", Tag::String("Hampus".into())), ("value", Tag::Float(0.75))]),
            ),
          ]),
        ),
        ("byteTest", Tag::Byte(127)),
        ("shortTest", Tag::Short(32767)),
        ("intTest", Tag::Int(2147483647)),
        ("longTest", Tag::Long(9223372036854775807)),
        ("floatTest", Tag::Float(0.498_231_47)),
        ("doubleTest", Tag::Double(0.493_128_713_218_231_5)),
        (
          "stringTest",
          Tag::String(
            "HELLO WORLD THIS IS A TEST STRING ÅÄÖ!".into(),
          ),
        ),
        (
          "listTest (long)",
          Tag::List(vec![
            Tag::Long(11),
            Tag::Long(12),
            Tag::Long(13),
            Tag::Long(14),
            Tag::Long(15),
          ]),
        ),
        (
          "listTest (compound)",
          Tag::List(vec![
            Tag::new_compound(&[
              ("created-on", Tag::Long(1264099775885)),
              ("name", Tag::String("Compound tag #0".into())),
            ]),
            Tag::new_compound(&[
              ("created-on", Tag::Long(1264099775885)),
              ("name", Tag::String("Compound tag #1".into())),
            ]),
          ]),
        ),
        (
          "byteArrayTest (the first 1000 values of (n*n*255+n*7)%100, starting with n=0 (0, 62, 34, 16, 8, ...))",
          Tag::ByteArr({
            let mut out = vec![0; 1000];
            for n in 0..1000 {
              out[n] = ((n * n * 255 + n * 7) % 100) as u8;
            }
            out
          }),
        ),
      ]),
    );
    // let mut data = vec![];
    // let mut decoder =
    // GzDecoder::new(&include_bytes!("../../../../data/nbt/bigtest.nbt")[..]);
    // decoder.read_to_end(&mut data).unwrap();
    // dbg!(&data);
    // let v = NBT::deserialize(data)?;
    //
    // // More readable errors
    // let expected_map = expected.compound();
    // for (name, val) in v.compound() {
    //   assert_eq!(&expected_map[name], val);
    // }
    // assert_eq!(v, expected);
    Ok(())
  }
}

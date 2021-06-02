use crate::util::Buffer;
use std::{collections::HashMap, error::Error, fmt, string::FromUtf8Error};

use super::{Tag, NBT};

#[derive(Debug)]
pub enum ParseError {
  InvalidType(u8),
  InvalidString(FromUtf8Error),
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidType(ty) => write!(f, "invalid tag type: {}", ty),
      Self::InvalidString(e) => write!(f, "invalid string: {}", e),
    }
  }
}

impl Error for ParseError {}

impl NBT {
  pub fn deserialize(buf: Vec<u8>) -> Result<Self, ParseError> {
    Self::deserialize_buf(&mut Buffer::new(buf))
  }
  fn deserialize_buf(buf: &mut Buffer) -> Result<Self, ParseError> {
    let ty = buf.read_u8();
    let len = buf.read_u16();
    let name = String::from_utf8(buf.read(len as usize)).unwrap();
    Ok(NBT::new(&name, Tag::deserialize(ty, buf)?))
  }
}

impl Tag {
  fn deserialize(ty: u8, buf: &mut Buffer) -> Result<Self, ParseError> {
    match ty {
      0 => Ok(Self::End),
      1 => Ok(Self::Byte(buf.read_i8())),
      2 => Ok(Self::Short(buf.read_i16())),
      3 => Ok(Self::Int(buf.read_i32())),
      4 => Ok(Self::Long(buf.read_i64())),
      5 => Ok(Self::Float(buf.read_f32())),
      6 => Ok(Self::Double(buf.read_f64())),
      7 => {
        let len = buf.read_i32();
        Ok(Self::ByteArr(buf.read(len as usize)))
      }
      8 => {
        let len = buf.read_i32();
        match String::from_utf8(buf.read(len as usize)) {
          Ok(v) => Ok(Self::String(v)),
          Err(e) => Err(ParseError::InvalidString(e)),
        }
      }
      9 => {
        let inner_ty = buf.read_u8();
        let len = buf.read_i32();
        let mut inner = Vec::with_capacity(len as usize);
        for _ in 0..len {
          inner.push(Tag::deserialize(inner_ty, buf)?);
        }
        Ok(Self::List(inner))
      }
      10 => {
        let mut inner = HashMap::new();
        loop {
          let ty = buf.read_u8();
          let len = buf.read_u16();
          let name = String::from_utf8(buf.read(len as usize)).unwrap();
          let tag = Tag::deserialize(ty, buf)?;
          if tag.ty() == Self::End.ty() {
            break;
          }
          inner.insert(name, tag);
        }
        Ok(Self::Compound(inner))
      }
      11 => {
        let len = buf.read_i32();
        let mut inner = Vec::with_capacity(len as usize);
        for _ in 0..len {
          inner.push(buf.read_i32());
        }
        Ok(Self::IntArray(inner))
      }
      12 => {
        let len = buf.read_i32();
        let mut inner = Vec::with_capacity(len as usize);
        for _ in 0..len {
          inner.push(buf.read_i64());
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
    let v = NBT::new("hello", Tag::compound(&[("small", Tag::Byte(5))]));
    let new = NBT::deserialize(v.serialize())?;
    assert_eq!(new, v);
    Ok(())
  }
}

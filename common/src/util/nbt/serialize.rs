use crate::util::Buffer;

use super::{Tag, NBT};

impl NBT {
  pub fn serialize(&self) -> Vec<u8> {
    let mut out = Buffer::new(vec![]);
    out.write_u8(self.tag.ty());
    out.write_u16(self.name.len() as u16);
    out.write_buf(self.name.as_bytes());
    out.write_buf(&self.tag.serialize());
    out.into_inner()
  }
}

impl Tag {
  /// Returns the type of the tag.
  fn ty(&self) -> u8 {
    match self {
      Self::End => 0,
      Self::Byte(_) => 1,
      Self::Short(_) => 2,
      Self::Int(_) => 3,
      Self::Long(_) => 4,
      Self::Float(_) => 5,
      Self::Double(_) => 6,
      Self::ByteArray(_) => 7,
      Self::String(_) => 8,
      Self::List(_) => 9,
      Self::Compound(_) => 10,
      Self::IntArray(_) => 11,
      Self::LongArray(_) => 12,
    }
  }

  /// Serializes the data of the tag. Does not add type byte.
  fn serialize(&self) -> Vec<u8> {
    let mut out = Buffer::new(vec![]);
    match self {
      Self::End => (),
      Self::Byte(v) => out.write_i8(*v),
      Self::Short(v) => out.write_i16(*v),
      Self::Int(v) => out.write_i32(*v),
      Self::Long(v) => out.write_i64(*v),
      Self::Float(v) => out.write_f32(*v),
      Self::Double(v) => out.write_f64(*v),
      Self::ByteArray(v) => {
        out.write_i32(v.len() as i32);
        out.write_buf(v);
      }
      Self::String(v) => {
        out.write_i32(v.len() as i32);
        out.write_buf(v.as_bytes());
      }
      Self::List(v) => {
        out.write_u8(v.get(0).unwrap_or(&Self::End).ty());
        out.write_i32(v.len() as i32);
        for tag in v {
          out.write_buf(&tag.serialize());
        }
      }
      Self::Compound(v) => {
        for nbt in v {
          out.write_buf(&nbt.serialize());
        }
      }
      Self::IntArray(v) => {
        out.write_i32(v.len() as i32);
        for elem in v {
          out.write_i32(*elem);
        }
      }
      Self::LongArray(v) => {
        out.write_i32(v.len() as i32);
        for elem in v {
          out.write_i64(*elem);
        }
      }
    }
    out.into_inner()
  }
}

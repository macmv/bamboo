use crate::util::Buffer;

use super::{Tag, NBT};

impl NBT {
  pub fn serialize_buf(&self, out: &mut Buffer<&mut Vec<u8>>) {
    out.write_u8(self.tag.ty());
    if matches!(self.tag, Tag::End) {
      return;
    }
    out.write_u16(self.name.len() as u16);
    out.write_buf(self.name.as_bytes());
    self.tag.serialize(out);
  }
  pub fn serialize(&self) -> Vec<u8> {
    let mut data = vec![];
    let mut out = Buffer::new(&mut data);
    self.serialize_buf(&mut out);
    data
  }
}

impl Tag {
  /// Returns the type of the tag.
  pub fn ty(&self) -> u8 {
    match self {
      Self::End => 0,
      Self::Byte(_) => 1,
      Self::Short(_) => 2,
      Self::Int(_) => 3,
      Self::Long(_) => 4,
      Self::Float(_) => 5,
      Self::Double(_) => 6,
      Self::ByteArr(_) => 7,
      Self::String(_) => 8,
      Self::List(_) => 9,
      Self::Compound(_) => 10,
      Self::IntArray(_) => 11,
      Self::LongArray(_) => 12,
    }
  }

  /// Serializes the data of the tag. Does not add type byte.
  fn serialize(&self, out: &mut Buffer<&mut Vec<u8>>) {
    match self {
      Self::End => (),
      Self::Byte(v) => out.write_i8(*v),
      Self::Short(v) => out.write_i16(*v),
      Self::Int(v) => out.write_i32(*v),
      Self::Long(v) => out.write_i64(*v),
      Self::Float(v) => out.write_f32(*v),
      Self::Double(v) => out.write_f64(*v),
      Self::ByteArr(v) => {
        out.write_i32(v.len() as i32);
        out.write_buf(v);
      }
      Self::String(v) => {
        out.write_u16(v.len() as u16);
        out.write_buf(v.as_bytes());
      }
      Self::List(v) => {
        out.write_u8(v.get(0).unwrap_or(&Self::End).ty());
        out.write_i32(v.len() as i32);
        for tag in v {
          tag.serialize(out);
        }
      }
      Self::Compound(v) => {
        for (name, tag) in v {
          // Each element in the HashMap is essentially a NBT, but we store it in a
          // seperated form, so we have a manual implementation of serialize() here.
          out.write_u8(tag.ty());
          if tag.ty() == Self::End.ty() {
            // End tags don't have a name, so we stop early.
            break;
          }
          out.write_u16(name.len() as u16);
          out.write_buf(name.as_bytes());
          tag.serialize(out);
        }
        out.write_u8(Self::End.ty());
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
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialize() {
    let _nbt = NBT::new("", Tag::compound(&[("MOTION_BLOCKING", Tag::LongArray(vec![5, 7]))]));
  }
}

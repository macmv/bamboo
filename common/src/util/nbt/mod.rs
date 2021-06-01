mod serialize;

#[derive(Debug)]
pub struct NBT {
  tag:  Tag,
  name: String,
}

#[derive(Debug)]
pub enum Tag {
  End,
  Byte(i8),
  Short(i16),
  Int(i32),
  Long(i64),
  Float(f32),
  Double(f64),
  ByteArr(Vec<u8>),
  String(String),
  List(Vec<Tag>),     // All elements must be the same type, and un-named.
  Compound(Vec<NBT>), // Types can be any kind, and named. Order is not defined.
  IntArray(Vec<i32>),
  LongArray(Vec<i64>),
}

impl NBT {
  pub fn new(name: &str, tag: Tag) -> Self {
    NBT { tag, name: name.into() }
  }

  /// Appends the given element to the list. This will panic if self is not a
  /// list, or if tag does not match the type of the existing elements.
  pub fn list_add(&mut self, tag: Tag) {
    if let Tag::List(inner) = &mut self.tag {
      if let Some(v) = inner.get(0) {
        if tag.ty() != v.ty() {
          panic!("cannot add different types to list. current: {:?}, new: {:?}", inner, tag);
        } else {
          inner.push(tag);
        }
      } else {
        // No elements yet, so we add this no matter what type it is.
        inner.push(tag);
      }
    } else {
      panic!("called list_add on non-list type: {:?}", self);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_list() {
    let mut list = NBT::new("List", Tag::List(vec![]));
    list.list_add(Tag::Int(5));
    list.list_add(Tag::Int(7));
  }
}

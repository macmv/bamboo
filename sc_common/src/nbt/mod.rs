mod deserialize;
mod serde;
mod serialize;

pub use self::serde::{to_nbt, to_tag};
pub use deserialize::ParseError;

use std::collections::HashMap;

/// This is an nbt tag. It has a name, and any amount of data. This can be used
/// to store item data, entity data, level data, and more.
#[derive(Debug, Clone, PartialEq)]
pub struct NBT {
  tag:  Tag,
  name: String,
}

impl Default for NBT {
  fn default() -> Self { NBT::new("", Tag::compound(&[])) }
}

/// This is a single tag. It does not contain a name, but has the actual data
/// for any of the nbt tags.
#[derive(Debug, Clone, PartialEq)]
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
  List(Vec<Tag>),                 // All elements must be the same type, and un-named.
  Compound(HashMap<String, Tag>), // Types can be any kind, and are named. Order is not defined.
  IntArray(Vec<i32>),
  LongArray(Vec<i64>),
}

impl NBT {
  /// Creates a new nbt tag. The tag value can be anything.
  ///
  /// # Panics
  /// This will panic if the tag is a list, and the values within that list
  /// contain multiple types. This is a limitation with the nbt data format:
  /// lists can only contain one type of data.
  pub fn new(name: &str, tag: Tag) -> Self {
    if let Tag::List(inner) = &tag {
      if let Some(v) = inner.get(0) {
        let ty = v.ty();
        for v in inner {
          if v.ty() != ty {
            panic!("the given list contains multiple types: {:?}", inner);
          }
        }
      }
    }
    NBT { tag, name: name.into() }
  }

  /// Creates an empty nbt tag.
  pub fn empty(name: &str) -> Self { NBT { tag: Tag::End, name: name.into() } }

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

  /// Appends the given element to the compound. This will panic if self is not
  /// a compound tag.
  pub fn compound_add(&mut self, name: String, value: Tag) {
    if let Tag::Compound(inner) = &mut self.tag {
      inner.insert(name, value);
    } else {
      panic!("called compound_add on non-compound type: {:?}", self);
    }
  }

  /// If this is a compound tag, this returns the inner data of the tag.
  /// Otherwise, this panics.
  pub fn compound(&self) -> &HashMap<String, Tag> {
    if let Tag::Compound(inner) = &self.tag {
      inner
    } else {
      panic!("called compound on non-compound type: {:?}", self);
    }
  }
}

impl Tag {
  /// A simpler way to construct compound tags inline.
  pub fn compound(value: &[(&str, Tag)]) -> Self {
    let mut inner = HashMap::new();
    for (name, tag) in value {
      inner.insert(name.to_string(), tag.clone());
    }
    Self::Compound(inner)
  }

  #[track_caller]
  pub fn unwrap_byte(&self) -> i8 {
    match self {
      Self::Byte(v) => *v,
      _ => panic!("not a byte: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_short(&self) -> i16 {
    match self {
      Self::Short(v) => *v,
      _ => panic!("not a short: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_int(&self) -> i32 {
    match self {
      Self::Int(v) => *v,
      _ => panic!("not an int: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_long(&self) -> i64 {
    match self {
      Self::Long(v) => *v,
      _ => panic!("not a long: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_float(&self) -> f32 {
    match self {
      Self::Float(v) => *v,
      _ => panic!("not a float: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_double(&self) -> f64 {
    match self {
      Self::Double(v) => *v,
      _ => panic!("not a double: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_string(&self) -> &str {
    match self {
      Self::String(v) => v,
      _ => panic!("not a string: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_byte_arr(&self) -> &[u8] {
    match self {
      Self::ByteArr(v) => v,
      _ => panic!("not a string: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_list(&self) -> &Vec<Tag> {
    match self {
      Self::List(v) => v,
      _ => panic!("not a list: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_compound(&self) -> &HashMap<String, Tag> {
    match self {
      Self::Compound(v) => v,
      _ => panic!("not a compound: {:?}", self),
    }
  }
  #[track_caller]
  pub fn unwrap_long_arr(&self) -> &Vec<i64> {
    match self {
      Self::LongArray(v) => v,
      _ => panic!("not a long array: {:?}", self),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_list() {
    let mut list = NBT::new("List", Tag::List(vec![Tag::Int(5), Tag::Int(6)]));
    list.list_add(Tag::Int(5));
    list.list_add(Tag::Int(7));
  }
}

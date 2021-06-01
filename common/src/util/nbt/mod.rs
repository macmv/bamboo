mod serialize;

use std::collections::HashSet;

pub enum NBT {
  End,
  Byte(i8),
  Short(i16),
  Int(i32),
  Long(i64),
  Float(f32),
  Double(f64),
  ByteArray(Vec<u8>),
  String(String),
  List(Vec<NBT>), // All elements must be the same type
  Compound(HashSet<NBT>),
  IntArray(Vec<i32>),
  LongArray(Vec<i64>),
}

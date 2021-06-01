mod serialize;

pub struct NBT {
  tag:  Tag,
  name: String,
}

enum Tag {
  End,
  Byte(i8),
  Short(i16),
  Int(i32),
  Long(i64),
  Float(f32),
  Double(f64),
  ByteArray(Vec<u8>),
  String(String),
  List(Vec<Tag>),     // All elements must be the same type, and un-named.
  Compound(Vec<NBT>), // Types can be any kind, and named. Order is not defined.
  IntArray(Vec<i32>),
  LongArray(Vec<i64>),
}

use super::tcp;
use crate::{
  util::{nbt::NBT, Item, UUID},
  ChunkPos, Pos,
};
use std::{
  collections::{HashMap, HashSet},
  hash::Hash,
};

/// A trait to deserialize data from a buffer. This is used in the protocol, to
/// simplify code generation.
pub trait ReadSc {
  fn read_sc(buf: &mut tcp::Packet) -> Self;
}
/// A trait to serialize data to a buffer. This is used in the protocol, to
/// simplify code generation.
pub trait WriteSc {
  fn write_sc(&self, buf: &mut tcp::Packet);
}

macro_rules! sc_simple {
  ($ty:ty, $read:ident, $write:ident) => {
    impl ReadSc for $ty {
      fn read_sc(buf: &mut tcp::Packet) -> Self {
        buf.$read()
      }
    }
    impl WriteSc for $ty {
      fn write_sc(&self, buf: &mut tcp::Packet) {
        buf.$write(*self)
      }
    }
  };
}

sc_simple!(bool, read_bool, write_bool);
sc_simple!(u8, read_u8, write_u8);
sc_simple!(i8, read_i8, write_i8);
sc_simple!(u16, read_u16, write_u16);
sc_simple!(i16, read_i16, write_i16);
sc_simple!(u32, read_u32, write_u32);
sc_simple!(i32, read_i32, write_i32);
sc_simple!(u64, read_u64, write_u64);
sc_simple!(i64, read_i64, write_i64);
sc_simple!(f32, read_f32, write_f32);
sc_simple!(f64, read_f64, write_f64);
sc_simple!(Pos, read_pos, write_pos);
sc_simple!(ChunkPos, read_chunk_pos, write_chunk_pos);
sc_simple!(UUID, read_uuid, write_uuid);

impl ReadSc for String {
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_str(32767)
  }
}
impl WriteSc for String {
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_str(self)
  }
}
impl ReadSc for Item {
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_item()
  }
}
impl WriteSc for Item {
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_item(self)
  }
}
impl ReadSc for NBT {
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_nbt()
  }
}
impl WriteSc for NBT {
  fn write_sc(&self, buf: &mut tcp::Packet) {
    // buf.write_nbt(self)
  }
}

impl<T> ReadSc for Option<T>
where
  T: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_option(|buf| buf.read_sc())
  }
}
impl<T> WriteSc for Option<T>
where
  T: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_option(self, |buf, v| buf.write_sc(v))
  }
}

impl<T> ReadSc for Vec<T>
where
  T: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_list(|buf| buf.read_sc())
  }
}
impl<T> WriteSc for Vec<T>
where
  T: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_list(self, |buf, v| buf.write_sc(v))
  }
}
impl<K, V> ReadSc for HashMap<K, V>
where
  K: Eq + Hash + ReadSc,
  V: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_map(|buf| buf.read_sc(), |buf| buf.read_sc())
  }
}
impl<K, V> WriteSc for HashMap<K, V>
where
  K: WriteSc + Eq + Hash,
  V: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_map(self, |buf, k| buf.write_sc(k), |buf, v| buf.write_sc(v))
  }
}
impl<T> ReadSc for HashSet<T>
where
  T: Eq + Hash + ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_set(|buf| buf.read_sc())
  }
}
impl<T> WriteSc for HashSet<T>
where
  T: WriteSc + Eq + Hash,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_set(self, |buf, k| k.write_sc(buf))
  }
}

impl<T> ReadSc for [T; 3]
where
  T: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    [buf.read_sc(), buf.read_sc(), buf.read_sc()]
  }
}
impl<T> WriteSc for [T; 3]
where
  T: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_sc(&self[0]);
    buf.write_sc(&self[1]);
    buf.write_sc(&self[2]);
  }
}
impl<T> ReadSc for [T; 4]
where
  T: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    [buf.read_sc(), buf.read_sc(), buf.read_sc(), buf.read_sc()]
  }
}
impl<T> WriteSc for [T; 4]
where
  T: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_sc(&self[0]);
    buf.write_sc(&self[1]);
    buf.write_sc(&self[2]);
    buf.write_sc(&self[3]);
  }
}
impl<T, U> ReadSc for (T, U)
where
  T: ReadSc,
  U: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    (buf.read_sc(), buf.read_sc())
  }
}
impl<T, U> WriteSc for (T, U)
where
  T: WriteSc,
  U: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_sc(&self.0);
    buf.write_sc(&self.1);
  }
}

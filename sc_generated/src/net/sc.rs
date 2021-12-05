use super::tcp;
use crate::{
  util::{nbt::NBT, Item, UUID},
  ChunkPos, Pos,
};
use sc_transfer::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};
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

impl MessageRead for Pos {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(Pos::new(m.read_i32()?, m.read_i32()?, m.read_i32()?))
  }
}
impl MessageWrite for Pos {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_i32(self.x)?;
    m.write_i32(self.y)?;
    m.write_i32(self.z)?;
    Ok(())
  }
}
impl MessageRead for ChunkPos {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(ChunkPos::new(m.read_i32()?, m.read_i32()?))
  }
}
impl MessageWrite for ChunkPos {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_i32(self.x())?;
    m.write_i32(self.z())?;
    Ok(())
  }
}
impl MessageRead for UUID {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(UUID::from_le_bytes(m.read_bytes(16)?.try_into().unwrap()))
  }
}
impl MessageWrite for UUID {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_bytes(&self.as_le_bytes())?;
    Ok(())
  }
}

impl MessageRead for Item {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Item::from_sc(m)
  }
}
impl MessageWrite for Item {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    self.to_sc(m)
  }
}
impl MessageRead for NBT {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    // TODO: ParseError into ReadError
    Ok(NBT::deserialize(m.read_buf()?).unwrap())
  }
}
impl MessageWrite for NBT {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_buf(&self.serialize())
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

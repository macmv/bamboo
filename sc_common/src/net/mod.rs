pub mod cb;
pub mod sb;

use crate::{
  math::{ChunkPos, Pos},
  util::{nbt::NBT, Item, UUID},
};
use sc_transfer::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};

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
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { Item::from_sc(m) }
}
impl MessageWrite for Item {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> { self.to_sc(m) }
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

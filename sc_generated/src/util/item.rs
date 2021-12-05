use super::nbt::NBT;
use sc_transfer::{MessageReader, MessageWriter, ReadError, WriteError};

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
  id:     i32,
  count:  u8,
  // Only exists on 1.8-1.12 clients. 1.13+ clients use NBT for this
  damage: i16,
  nbt:    NBT,
}

impl Item {
  pub fn new(id: i32, count: u8, damage: i16, nbt: NBT) -> Self {
    Item { id, count, damage, nbt }
  }
  pub fn from_sc(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(Item {
      id:     m.read_i32()?,
      count:  m.read_u8()?,
      damage: m.read_i16()?,
      nbt:    NBT::deserialize(m.read_buf()?).unwrap(),
    })
  }
  pub fn to_sc(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_i32(self.id)?;
    m.write_u8(self.count)?;
    m.write_i16(self.damage)?;
    m.write_buf(&self.nbt.serialize())?;
    Ok(())
  }

  pub fn id(&self) -> i32 {
    self.id
  }
  pub fn count(&self) -> u8 {
    self.count
  }
  pub fn nbt(&self) -> &NBT {
    &self.nbt
  }

  pub fn into_parts(self) -> (i32, u8, NBT) {
    (self.id, self.count, self.nbt)
  }
}

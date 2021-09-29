use super::nbt::NBT;
use crate::proto;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
  id:    i32,
  count: u8,
  nbt:   NBT,
}

impl Item {
  pub fn new(id: i32, count: u8, nbt: NBT) -> Self {
    Item { id, count, nbt }
  }
  pub fn from_proto(p: proto::Item) -> Self {
    Item { id: p.id, count: p.count as u8, nbt: NBT::deserialize(p.nbt).unwrap() }
  }
  pub fn to_proto(&self) -> proto::Item {
    proto::Item {
      present: self.id != -1,
      id:      self.id,
      count:   self.count.into(),
      nbt:     self.nbt.serialize(),
    }
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

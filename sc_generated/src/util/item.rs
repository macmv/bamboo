use crate::proto;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
  id:    i32,
  count: u8,
  nbt:   Vec<u8>,
}

impl Item {
  pub fn new(id: i32, count: u8, nbt: Vec<u8>) -> Self {
    Item { id, count, nbt }
  }
  pub fn from_proto(p: proto::Item) -> Self {
    Item { id: p.id, count: p.count as u8, nbt: p.nbt }
  }
  pub fn to_proto(&self) -> proto::Item {
    proto::Item {
      present: self.id != -1,
      id:      self.id,
      count:   self.count.into(),
      nbt:     self.nbt.clone(),
    }
  }

  pub fn id(&self) -> i32 {
    self.id
  }
  pub fn count(&self) -> u8 {
    self.count
  }
}

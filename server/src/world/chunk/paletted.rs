use super::section::Section as ChunkSection;

use common::{
  math::{Pos, PosError},
  proto,
};

pub struct Section {}

impl Default for Section {
  fn default() -> Self {
    Section {}
  }
}

impl Section {
  pub(super) fn new() -> Box<Self> {
    Box::new(Section {})
  }
}

impl ChunkSection for Section {
  fn set_block(&mut self, _pos: Pos, _ty: u32) -> Result<(), PosError> {
    Ok(())
  }
  fn get_block(&self, _pos: Pos) -> Result<u32, PosError> {
    Ok(0)
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section {})
  }
  fn to_latest_proto(&self) -> proto::chunk::Section {
    proto::chunk::Section::default()
  }
  fn to_old_proto(&self, f: &dyn Fn(u32) -> u32) -> proto::chunk::Section {
    proto::chunk::Section::default()
  }
}

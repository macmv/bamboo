use super::section::Section as ChunkSection;
use crate::block;

use common::{
  math::{Pos, PosError},
  proto,
};

pub struct Section {}

impl Section {
  pub(super) fn new() -> Box<dyn ChunkSection + Send> {
    Box::new(Section {})
  }
}

impl ChunkSection for Section {
  fn set_block(&mut self, _pos: Pos, _ty: &block::Type) -> Result<(), PosError> {
    Ok(())
  }
  fn get_block(&self, _pos: Pos) -> Result<block::Type, PosError> {
    Ok(block::Type::air())
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section {})
  }
  fn to_proto(&self) -> proto::chunk::Section {
    proto::chunk::Section::default()
  }
}

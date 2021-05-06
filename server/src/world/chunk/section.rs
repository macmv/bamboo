use crate::block;

use common::{
  math::{Pos, PosError},
  proto,
};

/// A chunk section.
pub trait Section {
  fn set_block(&mut self, pos: Pos, ty: &block::Type) -> Result<(), PosError>;
  fn get_block(&self, pos: Pos) -> Result<block::Type, PosError>;
  fn duplicate(&self) -> Box<dyn Section + Send>;
  fn to_proto(&self) -> proto::chunk::Section;
}

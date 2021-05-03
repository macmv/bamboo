use crate::block;

/// A chunk section.
pub trait Section {
  fn set_block(&mut self, pos: block::Pos, ty: block::Type) -> Result<(), block::PosError>;
  fn get_block(&self, pos: block::Pos) -> Result<block::Type, block::PosError>;
  fn duplicate(&self) -> Box<dyn Section + Send>;
}

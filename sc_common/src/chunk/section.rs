use crate::math::{Pos, PosError};
use std::any::Any;

/// A chunk section.
pub trait Section: Any {
  /// Creates an empty chunk section.
  fn new(max_bpe: u8) -> Self
  where
    Self: Sized;
  /// Sets a block within this chunk column. if the position is outside of the
  /// chunk column, it will return a PosError (even in release). The id is
  /// either a blockstate id (see [`block::Type`](crate::block::Type)) or a
  /// block id and metadata (for 1.8). Either way, it will always chop of the
  /// higher bits in the id. In release, this should be done silently, and in
  /// debug, this should panic.
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError>;
  /// This fills the chunk section with the given block. Min and max are
  /// inclusive coordinates, and min must be less than or equal to max. This
  /// function should only validate that if debug assertions are enabled.
  ///
  /// For fixed chunks, this is the same as calling set_block in a for loop.
  /// However, for paletted chunks, this can lead to large performance
  /// improvements.
  fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError>;
  /// This gets the block id at the given position. If the position is outside
  /// of the chunk column, it will return an error. If this chunk is <1.13, then
  /// it will return an number in the format `(id << 4) | meta`
  fn get_block(&self, pos: Pos) -> Result<u32, PosError>;
  /// Clones the entire chunk section. This is not `clone()`, because
  /// `[#derive(Clone)]` on structs that contain a Section should not clone an
  /// entire section.
  fn duplicate(&self) -> Box<dyn Section + Send>;

  fn set_from(&mut self, palette: Vec<u32>, data: Vec<u64>) {
    let _ = (palette, data);
    unimplemented!()
  }
}

use common::{
  math::{Pos, PosError},
  proto,
};

/// A chunk section.
pub trait Section {
  /// Sets a block within this chunk column. if the position is outside of the
  /// chunk column, it will return a PosError (even in release). The id is
  /// either a blockstate id (see [`block::Type`](crate::block::Type)) or a
  /// block id and metadata (for 1.8). Either way, it will always chop of the
  /// higher bits in the id. In release, this should be done silently, and in
  /// debug, this should panic.
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError>;
  /// This gets the block id at the given position. If the position is outside
  /// of the chunk column, it will return an error. If this chunk is <1.13, then
  /// it will return an number in the format `(id << 4) | meta`
  fn get_block(&self, pos: Pos) -> Result<u32, PosError>;
  /// Clones the entire chunk section. This is not `clone()`, because
  /// `[#derive(Clone)]` on structs that contain a Section should not clone an
  /// entire section.
  fn duplicate(&self) -> Box<dyn Section + Send>;
  /// Generates a protobuf from the given chunk column. Should only be used in
  /// `Chunk::to_proto`. This should have no effect on the chunk itself.
  fn to_latest_proto(&self) -> proto::chunk::Section;
  /// Generates a protobuf from the given chunk column. Should only be used in
  /// `Chunk::to_proto`. This should have no effect on the chunk itself. This
  /// will call f for every block id it encounters.
  fn to_old_proto(&self, f: &dyn Fn(u32) -> u32) -> proto::chunk::Section;
}

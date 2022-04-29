pub mod behavior;
mod material;
mod ty;
mod version;

pub use material::Material;
pub use ty::{Data, Kind, Prop, PropValue, Type};
pub use version::TypeConverter;

use bb_common::math::Pos;

/// A block in the worl. This simply stores a [`Type`] and a [`Pos`]. This
/// stores no references to the world, so this may be out of date.
pub struct Block {
  pub pos: Pos,
  pub ty:  Type,
}

impl Block {
  /// Creates a new block at the given position with the given type.
  pub fn new(pos: Pos, ty: Type) -> Self { Block { pos, ty } }
  /// Returns the kind of block.
  pub fn kind(&self) -> Kind { self.ty.kind() }
}

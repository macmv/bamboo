mod behavior;
mod material;
mod ty;
mod version;

pub use behavior::{Behavior, TileEntity};
pub use material::Material;
pub use ty::{Data, Kind, Prop, PropValue, Type};
pub use version::TypeConverter;

use crate::world::World;
use bb_common::math::Pos;
use behavior::BehaviorList;
use std::sync::Arc;

/// A block in the worl. This simply stores a [`Type`] and a [`Pos`]. This
/// stores no references to the world, so this may be out of date.
#[derive(Clone, Copy)]
pub struct Block<'a> {
  pub world: &'a Arc<World>,
  pub pos:   Pos,
  pub ty:    Type,
}

impl<'a> Block<'a> {
  /// Creates a new block at the given position with the given type.
  pub fn new(world: &'a Arc<World>, pos: Pos, ty: Type) -> Self { Block { world, pos, ty } }
  /// Returns the kind of block.
  pub fn kind(&self) -> Kind { self.ty.kind() }

  pub fn te<T: TileEntity, F: FnOnce(&T) -> R, R>(&self, f: F) -> R {
    let te_box = self
      .world
      .chunk(self.pos.chunk(), |c| c.get_te(self.pos.chunk_rel()))
      .unwrap_or_else(|| panic!("block at {} does not have tile entity", self.pos));
    let te = te_box
      .as_any()
      .downcast_ref::<T>()
      .unwrap_or_else(|| panic!("tile entity at {} has the wrong type", self.pos));
    f(te)
  }
}

pub struct BehaviorStore {
  behaviors: BehaviorList,
}

impl BehaviorStore {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { BehaviorStore { behaviors: behavior::make_behaviors() } }
  pub fn call<R>(&self, kind: Kind, f: impl FnOnce(&dyn Behavior) -> R) -> Option<R> {
    self.behaviors.get(kind).map(f)
  }
}

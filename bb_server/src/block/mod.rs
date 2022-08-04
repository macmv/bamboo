mod behavior;
mod custom;
mod material;
mod store;
mod ty;
mod version;

#[cfg(feature = "wasm_plugins")]
mod ffi;

pub use behavior::{Behavior, TileEntity};
pub use custom::{CustomData, CustomKind, CustomProp, CustomPropValue};
pub use material::Material;
pub use store::TypeStore;
pub use ty::{Data, ItemDrop, Kind, Prop, PropKind, PropValue, PropValueStore, Type};
pub use version::TypeConverter;

use crate::world::World;
use bb_common::math::Pos;
use behavior::BehaviorList;
use std::{fmt, sync::Arc};

/// A block in the worl. This simply stores a [`Type`] and a [`Pos`]. This
/// stores no references to the world, so this may be out of date.
#[derive(Clone, Copy)]
pub struct Block<'a> {
  pub world: &'a Arc<World>,
  pub pos:   Pos,
  pub ty:    Type<'a>,
}

impl fmt::Debug for Block<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("Block").field("pos", &self.pos).field("ty", &self.ty).finish()
  }
}

impl<'a> Block<'a> {
  /// Creates a new block at the given position with the given type.
  pub fn new(world: &'a Arc<World>, pos: Pos, ty: Type<'a>) -> Self { Block { world, pos, ty } }
  /// Returns the kind of block.
  pub fn kind(&self) -> Kind { self.ty.kind() }

  pub fn te<T: TileEntity, F: FnOnce(&T) -> R, R>(&self, f: F) -> R {
    let te_box = self
      .world
      .chunk(self.pos.chunk(), |c| c.get_te(self.pos.chunk_rel()).unwrap())
      .unwrap_or_else(|| panic!("block at {} does not have tile entity", self.pos));
    let te = te_box
      .as_any()
      .downcast_ref::<T>()
      .unwrap_or_else(|| panic!("tile entity at {} has the wrong type", self.pos));
    f(te)
  }

  pub fn set(&mut self, ty: Type<'a>) {
    self.world.set_block(self.pos, ty).unwrap();
    self.ty = ty;
  }
}

pub struct BehaviorStore {
  behaviors: BehaviorList,
}

impl BehaviorStore {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { BehaviorStore { behaviors: BehaviorList::new() } }
  pub fn call<R>(&self, kind: Kind, f: impl FnOnce(&dyn Behavior) -> R) -> R {
    self.behaviors.call(kind, f)
  }
}

use super::{Block, Data, Kind, Type};
use crate::{
  event::EventFlow,
  item::Stack,
  math::{Vec3, AABB},
  player::{BlockClick, Player},
  world::World,
};
use bb_common::math::{FPos, Pos};
use bb_transfer::{MessageReader, MessageWriter};
use std::{any::Any, sync::Arc};

mod impls;

pub trait Behavior: Send + Sync {
  /// Called when a block is about to be placed.
  ///
  /// This should handle things like logs rotating or torches not placing on
  /// ceilings.
  fn place<'a>(&self, data: &'a Data, pos: Pos, click: BlockClick) -> Type<'a> {
    let _ = (pos, click);
    data.default_type()
  }
  /// Called after this block is placed. The `block` is the block that was
  /// placed.
  ///
  /// This should handle falling blocks spawning after the block is placed.
  fn update_place(&self, world: &Arc<World>, block: Block) { let _ = (world, block); }
  /// Called whenever a block is updated next to `block`. `old` and `new` will
  /// both have the same position, and will be next to `block`.
  ///
  /// This should handle falling blocks being created after a block is broken
  /// underneath it.
  fn update(&self, world: &Arc<World>, block: Block, old: Block, new: Block) {
    let _ = (world, block, old, new);
  }
  /// Called when the block is placed. If the block needs to store extra
  /// information, a [`TileEntity`] should be returned.
  ///
  /// Blocks such as chests, juke boxes, and furnaces should return a tile
  /// entity here.
  ///
  /// If a block returns `Some` from this, it should also return `Some`
  /// from [`load_te`](Self::load_te).
  fn create_te(&self) -> Option<Arc<dyn TileEntity>> { None }
  /// Loads the tile entity for this block from the given message reader.
  ///
  /// If a block returns `Some` from `create_te`, it should return `Some`
  /// from this function.
  fn load_te(
    &self,
    r: &mut MessageReader,
  ) -> Option<Result<Arc<dyn TileEntity>, bb_transfer::ReadError>> {
    let _ = r;
    None
  }

  /// Called when a player right clicks on this block. If this returns `true`,
  /// the event was handled, and a block should not be placed.
  fn interact(&self, block: Block, player: &Arc<Player>) -> EventFlow {
    let _ = (block, player);
    EventFlow::Continue
  }
  /// Returns the drops for the given block. The default drops for this block
  /// are collected from the vanilla client, but this may require some
  /// overrides. Returning [`BlockDrops::Normal`] will use the vanilla drops,
  /// and returning [`BlockDrops::Custom`] will override the vanilla drops
  /// with the given [`BlockDrops`].
  fn drops(&self, block: Block) -> BlockDrops {
    let _ = block;
    BlockDrops::Normal
  }

  /// Returns the hitbox for this block. This hitbox should be relative the
  /// position 0, 0, 0. AABBs are centered, so a full block hitbox would be
  /// this:
  /// ```rust
  /// # use bb_server::math::{AABB, Vec3};
  /// # use bb_common::math::FPos;
  /// AABB::new(FPos::new(0.5, 0.0, 0.5), Vec3::new(1.0, 1.0, 1.0));
  /// ```
  fn hitbox(&self, block: Block) -> AABB {
    let data = block.world.block_converter().get(block.ty.kind());
    match data.bounding_box {
      super::ty::BoundingBoxKind::Block => {
        AABB::new(FPos::new(0.5, 0.0, 0.5), Vec3::new(1.0, 1.0, 1.0))
      }
      super::ty::BoundingBoxKind::Empty => {
        AABB::new(FPos::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0))
      }
    }
  }
}

struct DefaultBehavior;
impl Behavior for DefaultBehavior {}

pub trait TileEntity: Any + Send + Sync {
  fn save(&self, w: &mut MessageWriter<&mut Vec<u8>>) -> Result<(), bb_transfer::WriteError>;
  fn as_any(&self) -> &dyn Any;
}

#[derive(Default)]
pub struct BehaviorList {
  behaviors: Vec<Option<Box<dyn Behavior>>>,
}

impl BehaviorList {
  pub fn new() -> Self { BehaviorList::default() }
  // TODO: Use this in plugins with custom blocks
  #[allow(unused)]
  pub fn set(&mut self, kind: Kind, imp: Box<dyn Behavior>) {
    while kind.id() as usize >= self.behaviors.len() {
      self.behaviors.push(None);
    }
    self.behaviors[kind.id() as usize] = Some(imp);
  }
  pub fn call<R>(&self, kind: Kind, f: impl FnOnce(&dyn Behavior) -> R) -> R {
    if (kind.id() as usize) < self.behaviors.len() {
      if let Some(b) = &self.behaviors[kind.id() as usize] {
        return f(b.as_ref());
      }
    }
    bb_server_macros::behavior! {
      kind, f -> :Kind:

      *wood* = Oak, Birch, Spruce, DarkOak, Acacia, Jungle;
      *color* = White, Orange, Magenta, LightBlue, Yellow, Lime, Pink, Gray, LightGray, Cyan, Purple, Blue, Brown, Green, Red, Black;

      *wood*Log => impls::Log;
      Stripped*wood*Log => impls::Log;

      *wood*Trapdoor | WarpedTrapdoor => impls::Trapdoor;
      *wood*Door | WarpedDoor => impls::Door;

      *wood*Slab | StoneSlab | SmoothStoneSlab => impls::Slab;

      Sand | RedSand | Gravel => impls::Falling;

      CraftingTable => impls::CraftingTable;

      *color*Bed => impls::Bed;

      Chest => impls::Chest;

      _ => DefaultBehavior;
    }
  }
}

/// A collection of things to drop from a block or entity.
#[derive(Debug, Clone, Default)]
pub struct Drops {
  pub exp:   i32,
  pub items: Vec<Stack>,
}

pub enum BlockDrops {
  Normal,
  Custom(Drops),
}

impl Drops {
  pub fn empty() -> Self { Drops::default() }
}

/*
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn read_write_chest() {
    let te = impls::ChestTE::default();
    let mut data = vec![];
    let mut w = MessageWriter::new(&mut data);
    te.save(&mut w).unwrap();
    dbg!(&data);
    let mut r = MessageReader::new(&data);
    dbg!(&r);
    let te2 = impls::ChestTE::load(&mut r).unwrap();
    dbg!(te);
    dbg!(te2);
  }
}
*/

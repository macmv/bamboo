use super::{Block, Data, Kind, Type};
use crate::{player::Player, world::World};
use bb_common::{math::Pos, util::Face};
use std::{collections::HashMap, sync::Arc};

mod impls;

pub trait Behavior: Send + Sync {
  /// Called when a block is about to be placed.
  ///
  /// This should handle things like logs rotating or torches not placing on
  /// ceilings.
  fn place(&self, data: &Data, pos: Pos, face: Face) -> Type {
    let _ = (pos, face);
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
  fn create_tile_entity(&self) -> Option<Box<dyn TileEntity>> { None }

  /// Called when a player right clicks on this block. If this returns `true`,
  /// the event was handled, and a block should not be placed.
  fn interact(&self, block: Block, player: &Arc<Player>) -> bool {
    let _ = (block, player);
    false
  }
}

// TODO: This needs to be able to store it's data to disk.
pub trait TileEntity: Send {}

pub fn make_behaviors() -> HashMap<Kind, Box<dyn Behavior>> {
  let mut out: HashMap<_, Box<dyn Behavior>> = HashMap::new();
  macro_rules! behaviors {
    ( $($kind:ident $(| $kind2:ident)* => $impl:expr,)* ) => {
      $(
        out.insert(Kind::$kind, Box::new($impl));
        $(
          out.insert(Kind::$kind2, Box::new($impl));
        )*
      )*
    }
  }
  behaviors! {
    OakLog => impls::Log,
    BirchLog => impls::Log,
    SpruceLog => impls::Log,
    DarkOakLog => impls::Log,
    AcaciaLog => impls::Log,
    JungleLog => impls::Log,
    StrippedOakLog => impls::Log,
    StrippedBirchLog => impls::Log,
    StrippedSpruceLog => impls::Log,
    StrippedDarkOakLog => impls::Log,
    StrippedAcaciaLog => impls::Log,
    StrippedJungleLog => impls::Log,

    Sand | RedSand | Gravel => impls::Falling,

    CraftingTable => impls::CraftingTable,

    RedBed => impls::Bed,
  };
  out
}

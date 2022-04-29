use super::{Block, Data, Kind, Type};
use bb_common::{math::Pos, util::Face};
use std::collections::HashMap;

mod impls;

pub trait Behavior: Send + Sync {
  fn place(&self, data: &Data, pos: Pos, face: Face) -> Type {
    let _ = (pos, face);
    data.default_type()
  }
  fn update(&self, block: Block, old: Block, new: Block) { let _ = (block, old, new); }
  fn create_tile_entity(&self) -> Option<Box<dyn TileEntity>> { None }
}

// TODO: This needs to be able to store it's data to disk.
pub trait TileEntity: Send {}

pub fn make_behaviors() -> HashMap<Kind, Box<dyn Behavior>> {
  let mut out: HashMap<_, Box<dyn Behavior>> = HashMap::new();
  macro_rules! behaviors {
    ( $($kind:ident => $impl:expr,)* ) => {
      $(
        out.insert(Kind::$kind, Box::new($impl));
      )*
    }
  }
  behaviors! {
    OakLog => impls::LogBehavior,
    BirchLog => impls::LogBehavior,
    SpruceLog => impls::LogBehavior,
    DarkOakLog => impls::LogBehavior,
    AcaciaLog => impls::LogBehavior,
    JungleLog => impls::LogBehavior,
    StrippedOakLog => impls::LogBehavior,
    StrippedBirchLog => impls::LogBehavior,
    StrippedSpruceLog => impls::LogBehavior,
    StrippedDarkOakLog => impls::LogBehavior,
    StrippedAcaciaLog => impls::LogBehavior,
    StrippedJungleLog => impls::LogBehavior,
  };
  out
}

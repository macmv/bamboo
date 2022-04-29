use super::{Block, Kind, Type, TypeConverter};
use bb_common::{math::Pos, util::Face};

mod impls;

pub trait Behavior {
  fn place(&self, conv: &TypeConverter, kind: Kind, pos: Pos, face: Face) -> Type {
    let _ = (pos, face);
    conv.get(kind).default_type()
  }
  fn update(&self, block: Block, old: Block, new: Block) { let _ = (block, old, new); }
  fn create_tile_entity(&self) -> Option<Box<dyn TileEntity>> { None }
}

// TODO: This needs to be able to store it's data to disk.
pub trait TileEntity {}

pub fn default_for_kind(kind: Kind) -> Option<Box<dyn Behavior>> {
  Some(match kind {
    Kind::OakLog | Kind::BirchLog | Kind::AcaciaLog => Box::new(impls::LogBehavior),
    _ => return None,
  })
}

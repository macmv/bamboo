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
  out.insert(Kind::OakLog, Box::new(impls::LogBehavior));
  out.insert(Kind::BirchLog, Box::new(impls::LogBehavior));
  out.insert(Kind::SpruceLog, Box::new(impls::LogBehavior));
  out.insert(Kind::DarkOakLog, Box::new(impls::LogBehavior));
  out.insert(Kind::AcaciaLog, Box::new(impls::LogBehavior));
  out.insert(Kind::JungleLog, Box::new(impls::LogBehavior));
  out.insert(Kind::StrippedOakLog, Box::new(impls::LogBehavior));
  out.insert(Kind::StrippedBirchLog, Box::new(impls::LogBehavior));
  out.insert(Kind::StrippedSpruceLog, Box::new(impls::LogBehavior));
  out.insert(Kind::StrippedDarkOakLog, Box::new(impls::LogBehavior));
  out.insert(Kind::StrippedAcaciaLog, Box::new(impls::LogBehavior));
  out.insert(Kind::StrippedJungleLog, Box::new(impls::LogBehavior));
  out
}

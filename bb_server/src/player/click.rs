use super::Player;
use crate::{
  block::Block,
  math::{CollisionResult, Vec3},
};
use bb_common::util::Face;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct BlockClick<'a> {
  pub player: &'a Arc<Player>,
  pub dir:    Vec3,
  pub block:  Block<'a>,
  pub face:   Face,
}
#[derive(Debug, Clone, Copy)]
pub struct AirClick<'a> {
  pub player: &'a Arc<Player>,
  pub dir:    Vec3,
}

#[derive(Debug, Clone, Copy)]
pub enum Click<'a> {
  Air(AirClick<'a>),
  Block(BlockClick<'a>),
}

impl Click<'_> {
  pub fn do_raycast(&self, distance: f64, water: bool) -> Option<CollisionResult> {
    // TODO: Figure out eyes position
    let from = Vec3::from(self.player().pos()) + Vec3::new(0.0, 1.5, 0.0);
    let to = from + self.dir() * distance;

    self.player().world().raycast(from, to, water)
  }

  pub fn player(&self) -> &Arc<Player> {
    match self {
      Self::Air(air) => air.player,
      Self::Block(block) => block.player,
    }
  }
  pub fn dir(&self) -> Vec3 {
    match self {
      Self::Air(air) => air.dir,
      Self::Block(block) => block.dir,
    }
  }
}

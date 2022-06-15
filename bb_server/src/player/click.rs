use super::Player;
use crate::{
  block::Block,
  math::{CollisionResult, Vec3},
};
use bb_common::{math::FPos, util::Face};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct BlockClick<'a> {
  /// The player that click on a block.
  pub player: &'a Arc<Player>,
  /// The direction the player was looking when they clicked.
  pub dir:    Vec3,
  /// The block that was clicked on.
  pub block:  Block<'a>,
  /// The face that was clicked on. For example, `Face::Top` means the player
  /// was looking at the top face of a block.
  pub face:   Face,
  /// The position within the face. Relative to the minimum corner, so this will
  /// be within 0 - 1 on all axis.
  pub cursor: FPos,
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
  pub fn do_raycast(&self, distance: f64, water: bool) -> Option<(FPos, CollisionResult)> {
    // TODO: Figure out eyes position
    let from = self.player().pos() + FPos::new(0.0, 1.5, 0.0);
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

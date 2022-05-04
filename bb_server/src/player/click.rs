use super::Player;
use crate::math::{CollisionResult, Vec3};
use bb_common::util::Face;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct Click<'a> {
  pub face:   Face,
  pub dir:    Vec3,
  pub player: &'a Arc<Player>,
}

impl Click<'_> {
  pub fn do_raycast(&self, distance: f64, water: bool) -> Option<CollisionResult> {
    // TODO: Figure out eyes position
    let from = Vec3::from(self.player.pos()) + Vec3::new(0.0, 1.5, 0.0);
    let to = from + self.dir * distance;

    self.player.world().raycast(from, to, water)
  }
}

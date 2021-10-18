use super::World;
use crate::entity;
use sc_common::math::FPos;

impl World {
  pub fn summon(&self, ty: entity::Type, pos: FPos) -> i32 {
    let eid = self.eid();

    eid
  }
}

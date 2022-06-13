use crate::{block, player::Player};
use bb_common::math::{FPos, Pos, PosError};

pub struct World {
  wid: u32,
}

impl World {
  pub fn new(wid: u32) -> Self { World { wid } }

  pub fn get_block(&self, pos: Pos) -> Result<block::Type, PosError> {
    todo!();
  }
  pub fn set_block(&self, pos: Pos, ty: block::Type) {
    unsafe {
      bb_ffi::bb_world_set_block(
        self.wid,
        &bb_ffi::CPos { x: pos.x(), y: pos.y(), z: pos.z() },
        ty.id(),
      );
    }
  }
  pub fn players(&self) -> impl Iterator<Item = Player> {
    unsafe {
      let players = Box::from_raw(bb_ffi::bb_world_players(self.wid)).into_vec();
      players.into_iter().map(Player::new)
    }
  }
  pub fn raycast(&self, from: FPos, to: FPos, water: bool) -> Option<FPos> {
    unsafe {
      let ptr = bb_ffi::bb_world_raycast(
        &bb_ffi::CFPos { x: from.x(), y: from.y(), z: from.z() },
        &bb_ffi::CFPos { x: to.x(), y: to.y(), z: to.z() },
        bb_ffi::CBool::new(water),
      );
      if ptr.is_null() {
        None
      } else {
        let cpos = Box::from_raw(ptr);
        Some(FPos { x: cpos.x, y: cpos.y, z: cpos.z })
      }
    }
  }
}

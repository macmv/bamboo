use bb_common::math::Pos;

pub struct World {
  wid: u32,
}

impl World {
  pub fn new(wid: u32) -> Self { World { wid } }

  pub fn set_block(&self, pos: Pos, id: u32) {
    unsafe {
      bb_ffi::bb_world_set_block(
        self.wid,
        &bb_ffi::CPos { x: pos.x(), y: pos.y(), z: pos.z() },
        id,
      );
    }
  }
}

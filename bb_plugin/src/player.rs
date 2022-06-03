use crate::{particle::Particle, world::World, IntoFfi};
use bb_common::{math::FPos, util::UUID};
use bb_ffi::CUUID;

#[derive(Debug)]
pub struct Player {
  id: CUUID,
}

impl Player {
  pub fn new(id: CUUID) -> Self { Player { id } }

  pub fn world(&self) -> World {
    unsafe {
      let wid = bb_ffi::bb_player_world(&self.id);
      if wid >= 0 {
        World::new(wid as u32)
      } else {
        panic!()
      }
    }
  }
  pub fn id(&self) -> UUID {
    UUID::from_u128(
      (self.id.bytes[0] as u128)
        | (self.id.bytes[1] as u128) << (1 * 32)
        | (self.id.bytes[2] as u128) << (2 * 32)
        | (self.id.bytes[3] as u128) << (3 * 32),
    )
  }
  pub fn username(&self) -> String {
    unsafe {
      let cstr = Box::from_raw(bb_ffi::bb_player_username(&self.id));
      cstr.into_string()
    }
  }
  pub fn send_particle(&self, particle: Particle) {
    unsafe {
      let cparticle = particle.into_ffi();
      bb_ffi::bb_player_send_particle(&self.id, &cparticle);
    }
  }
  pub fn pos(&self) -> FPos {
    unsafe {
      let cpos = Box::leak(Box::from_raw(bb_ffi::bb_player_pos(&self.id)));
      FPos { x: cpos.x, y: cpos.y, z: cpos.z }
    }
  }
  pub fn look_as_vec(&self) -> FPos {
    unsafe {
      let cpos = Box::from_raw(bb_ffi::bb_player_look_as_vec(&self.id));
      FPos { x: cpos.x, y: cpos.y, z: cpos.z }
    }
  }
}

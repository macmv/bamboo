use crate::{particle::Particle, world::World, IntoFfi};
use bb_ffi::CUUID;
use std::ffi::CStr;

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
  pub fn username(&self) -> String {
    unsafe {
      let mut buf = [0; 64];
      // We need null terminator, so we make use this doesn't overwrite the last byte.
      bb_ffi::bb_player_username(&self.id, buf.as_mut_ptr(), buf.len() as u32 - 1);
      CStr::from_ptr(buf.as_ptr() as *const _).to_str().unwrap().into()
    }
  }
  pub fn send_particle(&self, particle: Particle) {
    unsafe {
      let cparticle = particle.into_ffi();
      bb_ffi::bb_player_send_particle(&self.id, &cparticle);
    }
  }
}

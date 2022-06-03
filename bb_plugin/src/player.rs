use crate::{particle::Particle, world::World, IntoFfi};
use bb_ffi::CUUID;

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
}

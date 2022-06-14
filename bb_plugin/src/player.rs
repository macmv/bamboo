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
  /// Returns the id of this player.
  ///
  /// This will always return their UUID, even if this player has disconnected.
  pub fn id(&self) -> UUID {
    UUID::from_u128(
      (self.id.bytes[0] as u128)
        | (self.id.bytes[1] as u128) << 32
        | (self.id.bytes[2] as u128) << (2 * 32)
        | (self.id.bytes[3] as u128) << (3 * 32),
    )
  }
  /// Returns the username of this player.
  pub fn username(&self) -> String {
    // TODO: What to do if the player has disconnected?
    unsafe {
      let cstr = Box::from_raw(bb_ffi::bb_player_username(&self.id));
      cstr.into_string()
    }
  }
  /// Spawns a particle for this player. Other players will not be able to see
  /// this particle.
  ///
  /// This will do nothing if the player has logged off.
  pub fn send_particle(&self, particle: Particle) {
    unsafe {
      let cparticle = particle.into_ffi();
      bb_ffi::bb_player_send_particle(&self.id, &cparticle);
    }
  }
  /// Returns the player's position.
  pub fn pos(&self) -> FPos {
    // TODO: What to do if the player has disconnected?
    unsafe {
      let cpos = Box::from_raw(bb_ffi::bb_player_pos(&self.id));
      FPos { x: cpos.x, y: cpos.y, z: cpos.z }
    }
  }
  /// Returns the player's looking direction, as a unit vector.
  pub fn look_as_vec(&self) -> FPos {
    // TODO: What to do if the player has disconnected?
    unsafe {
      let cpos = Box::from_raw(bb_ffi::bb_player_look_as_vec(&self.id));
      FPos { x: cpos.x, y: cpos.y, z: cpos.z }
    }
  }
}

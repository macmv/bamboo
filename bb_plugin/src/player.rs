use crate::{math::Vec3, particle::Particle, world::World, FromFfi, IntoFfi};
use bb_common::{math::FPos, util::UUID};
use bb_ffi::CUUID;

#[derive(Debug)]
pub struct Player {
  id: UUID,
}

impl FromFfi for Player {
  type Ffi = CUUID;

  fn from_ffi(c: CUUID) -> Self { Player { id: UUID::from_ffi(c) } }
}

impl Player {
  /// Returns the world this player is in.
  pub fn world(&self) -> World {
    // TODO: What to do if the player has disconnected?
    unsafe {
      let wid = bb_ffi::bb_player_world(&self.id.into_ffi());
      if wid >= 0 {
        World::new(wid as u32)
      } else {
        panic!("player is not in a world")
      }
    }
  }
  /// Returns the id of this player.
  ///
  /// This will always return their UUID, even if this player has disconnected.
  pub fn id(&self) -> UUID { self.id }
  /// Returns the username of this player.
  pub fn username(&self) -> String {
    // TODO: What to do if the player has disconnected?
    unsafe {
      let cstr = Box::from_raw(bb_ffi::bb_player_username(&self.id.into_ffi()));
      cstr.into_string()
    }
  }
  /// Spawns a particle for this player. Other players will not be able to see
  /// this particle.
  ///
  /// This will do nothing if the player has logged off.
  pub fn send_particle(&self, particle: Particle) {
    unsafe {
      bb_ffi::bb_player_send_particle(&self.id.into_ffi(), &particle.into_ffi());
    }
  }
  /// Returns the player's position.
  pub fn pos(&self) -> FPos {
    // TODO: What to do if the player has disconnected?
    unsafe {
      let cpos = Box::from_raw(bb_ffi::bb_player_pos(&self.id.into_ffi()));
      FPos::from_ffi(*cpos)
    }
  }
  /// Returns the player's looking direction, as a unit vector.
  pub fn look_as_vec(&self) -> Vec3 {
    // TODO: What to do if the player has disconnected?
    unsafe {
      let cpos = Box::from_raw(bb_ffi::bb_player_look_as_vec(&self.id.into_ffi()));
      Vec3::from_ffi(*cpos)
    }
  }
}

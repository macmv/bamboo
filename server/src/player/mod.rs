use common::math::UUID;

use crate::net::Connection;

pub struct Player {
  // The EID of the player. Never changes.
  id:       u32,
  // Player's username
  username: String,
  uuid:     UUID,
  conn:     Connection,
}

impl Player {
  pub fn new(id: u32, username: String, uuid: UUID, conn: Connection) -> Self {
    Player { id, username, uuid, conn }
  }
}

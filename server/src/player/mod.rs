use common::math::UUID;
use std::sync::Arc;
use tonic::Status;

use crate::net::Connection;

pub struct Player {
  // The EID of the player. Never changes.
  id:       u32,
  // Player's username
  username: String,
  uuid:     UUID,
  conn:     Arc<Connection>,
}

impl Player {
  pub fn new(id: u32, username: String, uuid: UUID, conn: Arc<Connection>) -> Self {
    Player { id, username, uuid, conn }
  }

  /// Returns the player's username.
  pub fn username(&self) -> &str {
    &self.username
  }
  /// Returns the connection that this player is connected on. This can be used
  /// to check if the player has disconnected.
  pub fn conn(&self) -> &Connection {
    &self.conn
  }
}

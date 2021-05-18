use std::sync::Arc;

use common::{math::UUID, version::ProtocolVersion};

use crate::{net::Connection, world::World};

pub struct Player {
  // The EID of the player. Never changes.
  _id:      i32,
  // Player's username
  username: String,
  _uuid:    UUID,
  conn:     Arc<Connection>,
  ver:      ProtocolVersion,
  world:    Arc<World>,
}

impl Player {
  pub fn new(
    id: i32,
    username: String,
    uuid: UUID,
    conn: Arc<Connection>,
    ver: ProtocolVersion,
    world: Arc<World>,
  ) -> Self {
    Player { _id: id, username, _uuid: uuid, conn, ver, world }
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

  /// Returns the version that this player is on.
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }

  /// Returns a reference to the world the player is in.
  pub fn world(&self) -> &World {
    &self.world
  }
}

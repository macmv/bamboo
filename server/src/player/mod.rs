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

  x: f64,
  y: f64,
  z: f64,

  next_x: f64,
  next_y: f64,
  next_z: f64,

  yaw:   f32,
  pitch: f32,

  next_yaw:   f32,
  next_pitch: f32,
}

impl Player {
  pub fn new(
    id: i32,
    username: String,
    uuid: UUID,
    conn: Arc<Connection>,
    ver: ProtocolVersion,
    world: Arc<World>,
    x: f64,
    y: f64,
    z: f64,
  ) -> Self {
    Player {
      _id: id,
      username,
      _uuid: uuid,
      conn,
      ver,
      world,
      x,
      y,
      z,
      next_x: x,
      next_y: y,
      next_z: z,
      yaw: 0.0,
      pitch: 0.0,
      next_yaw: 0.0,
      next_pitch: 0.0,
    }
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

  /// This will move the player on the next player tick. Used whenever a
  /// position packet is recieved.
  pub(crate) fn set_next_pos(&mut self, x: f64, y: f64, z: f64) {
    self.next_x = x;
    self.next_y = y;
    self.next_z = z;
  }

  /// This will set the player's look direction on the next player tick. Used
  /// whenever a player look packet is recieved.
  pub(crate) fn set_next_look(&mut self, yaw: f32, pitch: f32) {
    self.next_yaw = yaw;
    self.next_pitch = pitch;
  }

  /// Updates the player's position/velocity. This will apply gravity, and do
  /// collision checks. Should never be called at a different rate than the
  /// global tick rate.
  pub(crate) fn tick(&mut self) {
    // TODO: Movement checks
    self.x = self.next_x;
    self.y = self.next_y;
    self.z = self.next_z;
    self.yaw = self.next_yaw;
    self.pitch = self.next_pitch;
  }
}

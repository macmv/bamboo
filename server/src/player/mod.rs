use std::{fmt, sync::Arc};

use common::{math::UUID, net::cb, util::Chat, version::ProtocolVersion};

use crate::{item::Inventory, net::Connection, world::World};

pub struct Player {
  // The EID of the player. Never changes.
  _id:      i32,
  // Player's username
  username: String,
  uuid:     UUID,
  conn:     Arc<Connection>,
  ver:      ProtocolVersion,
  world:    Arc<World>,

  inventory: Inventory,

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

impl fmt::Debug for Player {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Player")
      .field("username", &self.username)
      .field("uuid", &self.uuid)
      .field("ver", &self.ver)
      .field("inventory", &self.inventory)
      .field("x", &self.x)
      .field("y", &self.x)
      .field("z", &self.x)
      .field("yaw", &self.x)
      .field("pitch", &self.x)
      .finish()
  }
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
      uuid,
      conn,
      ver,
      world,
      inventory: Inventory::new(46), // This is 45 on 1.8, because there was no off hand.
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
  /// Returns the connection that this player is using. This can be used to
  /// check if the player has disconnected.
  pub fn conn(&self) -> &Connection {
    &self.conn
  }
  /// Returns the connection that this player is using. This will clone the
  /// internal Arc that is used to store the connection.
  pub(crate) fn clone_conn(&self) -> Arc<Connection> {
    self.conn.clone()
  }

  /// Returns the version that this player is on.
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }

  /// Returns a reference to the player's inventory.
  pub fn inventory(&self) -> &Inventory {
    &self.inventory
  }
  // Returns a mutable reference to the player's inventory.
  pub fn inventory_mut(&mut self) -> &mut Inventory {
    &mut self.inventory
  }

  /// Returns a reference to the world the player is in.
  pub fn world(&self) -> &World {
    &self.world
  }
  /// Returns a cloned reference to the world that the player is in.
  pub fn clone_world(&self) -> Arc<World> {
    self.world.clone()
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

  pub async fn send_message(&self, msg: &Chat) {
    let mut out = cb::Packet::new(cb::ID::Chat);
    out.set_str("message", msg.to_json());
    out.set_byte("position", 0); // Chat box, not over hotbar
    self.conn().send(out).await;
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

  /// Returns the player's X position. This is only updated once per tick.
  pub fn x(&self) -> f64 {
    self.x
  }
  /// Returns the player's Y position. This is only updated once per tick.
  pub fn y(&self) -> f64 {
    self.y
  }
  /// Returns the player's Z position. This is only updated once per tick.
  pub fn z(&self) -> f64 {
    self.z
  }
  /// Returns the player's yaw angle. This is the amount that they are looking
  /// to the side. It is in the range -180-180. This is only updated once per
  /// tick.
  pub fn yaw(&self) -> f32 {
    self.yaw
  }
  /// Returns the player's pitch angle. This is the amount that they are looking
  /// up or down. It is within the range -90..90. This is only updated once per
  /// tick.
  pub fn pitch(&self) -> f32 {
    self.pitch
  }
}

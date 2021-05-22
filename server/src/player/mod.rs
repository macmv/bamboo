use std::{
  fmt,
  sync::{Arc, Mutex, MutexGuard},
};

use common::{math::UUID, net::cb, util::Chat, version::ProtocolVersion};

use crate::{item::Inventory, net::Connection, world::World};

#[derive(Debug)]
struct PlayerPosition {
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

pub struct Player {
  // The EID of the player. Never changes.
  eid:      i32,
  // Player's username
  username: String,
  uuid:     UUID,
  conn:     Arc<Connection>,
  ver:      ProtocolVersion,
  world:    Arc<World>,

  inventory: Mutex<Inventory>,

  pos: Mutex<PlayerPosition>,
}

impl fmt::Debug for Player {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Player")
      .field("username", &self.username)
      .field("uuid", &self.uuid)
      .field("ver", &self.ver)
      .field("inventory", &self.inventory)
      .field("pos", &self.pos)
      .finish()
  }
}

impl Player {
  pub fn new(
    eid: i32,
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
      eid,
      username,
      uuid,
      conn,
      ver,
      world,
      // This is 45 on 1.8, because there was no off hand.
      inventory: Mutex::new(Inventory::new(46)),
      pos: Mutex::new(PlayerPosition {
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
      }),
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
  /// Returns the player's entity id. Used to send packets about entities.
  pub fn eid(&self) -> i32 {
    self.eid
  }
  /// Returns the player's uuid. Used to lookup players in the world.
  pub fn id(&self) -> UUID {
    self.uuid
  }

  /// Returns the version that this player is on.
  pub fn ver(&self) -> ProtocolVersion {
    self.ver
  }

  /// Returns a locked reference to the player's inventory.
  pub fn lock_inventory(&self) -> MutexGuard<Inventory> {
    self.inventory.lock().unwrap()
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
  pub(crate) fn set_next_pos(&self, x: f64, y: f64, z: f64) {
    let mut pos = self.pos.lock().unwrap();
    pos.next_x = x;
    pos.next_y = y;
    pos.next_z = z;
  }

  /// This will set the player's look direction on the next player tick. Used
  /// whenever a player look packet is recieved.
  pub(crate) fn set_next_look(&self, yaw: f32, pitch: f32) {
    let mut pos = self.pos.lock().unwrap();
    pos.next_yaw = yaw;
    pos.next_pitch = pitch;
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
  pub(crate) fn tick(&self) {
    let mut pos = self.pos.lock().unwrap();
    // TODO: Movement checks
    pos.x = pos.next_x;
    pos.y = pos.next_y;
    pos.z = pos.next_z;
    pos.yaw = pos.next_yaw;
    pos.pitch = pos.next_pitch;
  }

  /// Returns the player's position. This is only updated once per tick.
  pub fn pos(&self) -> (f64, f64, f64) {
    let pos = self.pos.lock().unwrap();
    (pos.x, pos.y, pos.z)
  }
  /// Returns the player's pitch and yaw angle. This is the amount that they are
  /// looking to the side. It is in the range -180-180. This is only updated
  /// once per tick.
  pub fn rotation(&self) -> (f32, f32) {
    let pos = self.pos.lock().unwrap();
    (pos.pitch, pos.yaw)
  }
}

#[test]
fn assert_sync() {
  fn is_sync<T: Send + Sync>() {}
  is_sync::<Player>(); // only compiles is player is Sync
}

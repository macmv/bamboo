use std::{
  cmp,
  cmp::Ordering,
  fmt,
  sync::{Arc, Mutex, MutexGuard},
};

use common::{
  math::{ChunkPos, FPos, UUID},
  net::cb,
  util::Chat,
  version::ProtocolVersion,
};

use crate::{
  item::{Inventory, Stack},
  net::Connection,
  world::World,
};

#[derive(Debug)]
struct PlayerPosition {
  // This is the current position of the player. It is only updated once per tick.
  curr: FPos,

  // This is the position on the previous tick. It is only updated once per tick.
  prev: FPos,

  // This is the most recently recieved position packet. It is updated whenever a position packet
  // is recieved. It is also used to set x,y,z on the next tick.
  next: FPos,

  yaw:   f32,
  pitch: f32,

  next_yaw:   f32,
  next_pitch: f32,
}

#[derive(Debug)]
pub struct PlayerInventory {
  inv:            Inventory,
  // An index into the hotbar (0..=8)
  selected_index: u8,
}

impl PlayerInventory {
  pub fn new() -> Self {
    PlayerInventory { inv: Inventory::new(46), selected_index: 0 }
  }

  /// Returns the item in the player's main hand.
  pub fn main_hand(&self) -> &Stack {
    self.inv.get(self.selected_index as u32 + 36)
  }

  /// Returns the currently selected hotbar index.
  pub fn selected_index(&self) -> u8 {
    self.selected_index
  }

  /// Gets the item at the given index. 0 is part of the armor slots, not the
  /// start of the hotbar. To access the hotbar, add 36 to the index returned
  /// from main_hand.
  pub fn get(&self, index: u32) -> &Stack {
    self.inv.get(index)
  }

  /// Sets the item in the inventory.
  ///
  /// TODO: Send a packet here.
  pub fn set(&mut self, index: u32, stack: Stack) {
    self.inv.set(index, stack)
  }
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

  inv: Mutex<PlayerInventory>,

  pos: Mutex<PlayerPosition>,
}

impl fmt::Debug for Player {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Player")
      .field("username", &self.username)
      .field("uuid", &self.uuid)
      .field("ver", &self.ver)
      .field("inv", &self.inv)
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
    pos: FPos,
  ) -> Self {
    Player {
      eid,
      username,
      uuid,
      conn,
      ver,
      world,
      // This is 45 on 1.8, because there was no off hand.
      inv: Mutex::new(PlayerInventory::new()),
      pos: Mutex::new(PlayerPosition {
        curr:       pos,
        prev:       pos,
        next:       pos,
        yaw:        0.0,
        pitch:      0.0,
        next_yaw:   0.0,
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
  pub fn lock_inventory(&self) -> MutexGuard<PlayerInventory> {
    self.inv.lock().unwrap()
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
    pos.next = FPos::new(x, y, z);
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
  pub(crate) async fn tick(&self) {
    let old_chunk;
    let new_chunk;
    {
      let mut pos = self.pos.lock().unwrap();
      pos.prev = pos.curr;
      // TODO: Movement checks
      pos.curr = pos.next;
      pos.yaw = pos.next_yaw;
      pos.pitch = pos.next_pitch;
      // Whether or not the collision checks passes, we now have a movement
      // vector; from prev to curr.
      old_chunk = pos.prev.block().chunk();
      new_chunk = pos.curr.block().chunk();
    }
    if old_chunk != new_chunk {
      let view_distance = 10; // TODO: Listen for client settings on this
      let delta = new_chunk - old_chunk;
      let new_top_left = new_chunk - ChunkPos::new(view_distance, view_distance);
      let new_bot_right = new_chunk + ChunkPos::new(view_distance, view_distance);
      let old_top_left = old_chunk - ChunkPos::new(view_distance, view_distance);
      let old_bot_right = old_chunk + ChunkPos::new(view_distance, view_distance);
      // Sides (including corners)
      match delta.x().cmp(&0) {
        Ordering::Greater => {
          self
            .load_chunks(
              ChunkPos::new(new_bot_right.x(), new_top_left.z()),
              ChunkPos::new(old_bot_right.x(), new_bot_right.z()),
            )
            .await;
          self
            .unload_chunks(
              ChunkPos::new(old_top_left.x(), old_top_left.z()),
              ChunkPos::new(new_top_left.x(), old_bot_right.z()),
            )
            .await;
        }
        Ordering::Less => {
          self
            .load_chunks(
              ChunkPos::new(old_top_left.x(), new_top_left.z()),
              ChunkPos::new(new_top_left.x(), new_bot_right.z()),
            )
            .await;
          self
            .unload_chunks(
              ChunkPos::new(new_bot_right.x(), old_top_left.z()),
              ChunkPos::new(old_bot_right.x(), old_bot_right.z()),
            )
            .await;
        }
        _ => {}
      };
      // Top/Bottom (excluding corners)
      match delta.z().cmp(&0) {
        Ordering::Greater => {
          self
            .load_chunks(
              ChunkPos::new(cmp::max(old_top_left.x(), new_top_left.x()), old_bot_right.z()),
              ChunkPos::new(cmp::min(old_bot_right.x(), new_bot_right.x()), new_bot_right.z()),
            )
            .await;
          self
            .unload_chunks(
              ChunkPos::new(cmp::max(old_top_left.x(), new_top_left.x()), old_top_left.z()),
              ChunkPos::new(cmp::min(old_bot_right.x(), new_bot_right.x()), new_top_left.z()),
            )
            .await;
        }
        Ordering::Less => {
          self
            .load_chunks(
              ChunkPos::new(cmp::max(old_top_left.x(), new_top_left.x()), old_top_left.z()),
              ChunkPos::new(cmp::min(old_bot_right.x(), new_bot_right.x()), new_top_left.z()),
            )
            .await;
          self
            .unload_chunks(
              ChunkPos::new(cmp::max(old_top_left.x(), new_top_left.x()), old_bot_right.z()),
              ChunkPos::new(cmp::min(old_bot_right.x(), new_bot_right.x()), new_bot_right.z()),
            )
            .await;
        }
        _ => {}
      };
    }
  }

  async fn load_chunks(&self, min: ChunkPos, max: ChunkPos) {
    for x in min.x()..max.x() {
      for z in min.z()..max.z() {
        self.conn.send(self.world.serialize_chunk(ChunkPos::new(x, z), self.ver().block())).await;
      }
    }
  }
  async fn unload_chunks(&self, min: ChunkPos, max: ChunkPos) {
    for x in min.x()..max.x() {
      for z in min.z()..max.z() {
        let mut out = cb::Packet::new(cb::ID::UnloadChunk);
        out.set_i32("chunk_x", x);
        out.set_i32("chunk_z", z);
        self.conn.send(out).await;
      }
    }
  }

  /// Returns the player's position. This is only updated once per tick. This
  /// also needs to lock a mutex, so you should not call it very often.
  pub fn pos(&self) -> FPos {
    let pos = self.pos.lock().unwrap();
    pos.curr
  }
  /// Returns the player's current and previous position. This is only updated
  /// once per tick. This needs to lock a mutex, so if you need the player's
  /// previous position, it is better to call this without calling
  /// [`pos`](Self::pos). The first item returned is the current position, and
  /// the second item is the previous position.
  pub fn pos_with_prev(&self) -> (FPos, FPos) {
    let pos = self.pos.lock().unwrap();
    (pos.curr, pos.prev)
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

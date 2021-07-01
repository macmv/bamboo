use std::{
  cmp,
  cmp::Ordering,
  fmt,
  sync::{Arc, Mutex, MutexGuard},
};

use common::{
  math::{ChunkPos, FPos},
  net::cb,
  util::{Chat, UUID},
  version::ProtocolVersion,
};

use crate::{
  entity::Metadata,
  item::{Inventory, Stack},
  net::Connection,
  world::World,
};

#[derive(Debug, Clone)]
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

  /// Sets the selected index. Should only be used when recieving a held item
  /// slot packet.
  pub(crate) fn set_selected(&mut self, index: u8) {
    self.selected_index = index;
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

  /// Sends the player a chat message.
  pub async fn send_message(&self, msg: &Chat) {
    let mut out = cb::Packet::new(cb::ID::Chat);
    out.set_str("message", msg.to_json());
    out.set_byte("position", 0); // Chat box, not system message or over hotbar
    self.conn().send(out).await;
  }
  /// Sends the player a chat message, which will appear over their hotbar.
  pub async fn send_hotbar(&self, msg: &Chat) {
    let mut out = cb::Packet::new(cb::ID::Chat);
    out.set_str("message", msg.to_json());
    out.set_byte("position", 2); // Hotbar, not chat box or system message
    self.conn().send(out).await;
  }

  /// Updates the player's position/velocity. This will apply gravity, and do
  /// collision checks. Should never be called at a different rate than the
  /// global tick rate.
  pub(crate) async fn tick(&self) {
    let old_chunk;
    let new_chunk;
    let look_changed;
    let pos_changed;
    let pos = {
      let mut pos = self.pos.lock().unwrap();
      pos.prev = pos.curr;
      look_changed = pos.yaw != pos.next_yaw || pos.pitch != pos.next_pitch;
      pos_changed = pos.curr != pos.next;
      // TODO: Movement checks here
      pos.curr = pos.next;
      pos.yaw = pos.next_yaw;
      // We want to keep yaw within 0..=360
      pos.yaw = pos.yaw % 360.0;
      if pos.yaw < 0.0 {
        pos.yaw += 360.0;
      }
      pos.pitch = pos.next_pitch;
      // We want to clamp pitch between -90..=90
      if pos.pitch > 90.0 {
        pos.pitch = 90.0;
      } else if pos.pitch < -90.0 {
        pos.pitch = -90.0;
      }
      // Whether or not the collision checks passes, we now have a movement
      // vector; from prev to curr.
      old_chunk = pos.prev.block().chunk();
      new_chunk = pos.curr.block().chunk();
      pos.clone()
    };
    if pos_changed || look_changed {
      for other in self.world.players().await.iter().in_view(pos.curr.chunk()).not(self.uuid) {
        // Make player move for other
        let mut out;
        if pos_changed && look_changed {
          out = cb::Packet::new(cb::ID::EntityMoveLook);
        } else if pos_changed {
          out = cb::Packet::new(cb::ID::RelEntityMove);
        } else if look_changed {
          out = cb::Packet::new(cb::ID::EntityLook);
        } else {
          unreachable!();
        }
        out.set_int("entity_id", self.eid);
        if look_changed {
          info!("yaw: {}", pos.yaw);
          out.set_byte("yaw", (pos.yaw / 360.0 * 256.0).round() as u8);
          out.set_byte("pitch", (pos.pitch / 360.0 * 256.0).round() as i8 as u8);
        }
        if pos_changed {
          let mut dx = pos.curr.x() - pos.prev.x();
          let mut dy = pos.curr.y() - pos.prev.y();
          let mut dz = pos.curr.z() - pos.prev.z();
          let abs_pos = if other.ver() == ProtocolVersion::V1_8 {
            dx *= 32.0;
            dy *= 32.0;
            dz *= 32.0;
            if dx.abs() > i8::MAX.into() || dy.abs() > i8::MAX.into() || dz.abs() > i8::MAX.into() {
              true
            } else {
              // As truncates any negative floats to 0, but just copies the bits for i8 -> u8
              out.set_byte("d_x", dx.round() as i8 as u8);
              out.set_byte("d_y", dy.round() as i8 as u8);
              out.set_byte("d_z", dz.round() as i8 as u8);
              false
            }
          } else {
            dx *= 4096.0;
            dy *= 4096.0;
            dz *= 4096.0;
            // 32 * 128 * 8 = 16384, which is the max value of an i16. So if we have more
            // than an 8 block delta, we cannot send a relative movement packet.
            if dx.abs() > i16::MAX.into()
              || dy.abs() > i16::MAX.into()
              || dz.abs() > i16::MAX.into()
            {
              true
            } else {
              out.set_short("d_x", dx.round() as i16);
              out.set_short("d_y", dy.round() as i16);
              out.set_short("d_z", dz.round() as i16);
              false
            }
          };
          if abs_pos {
            out = cb::Packet::new(cb::ID::EntityTeleport);
            out.set_int("entity_id", self.eid);
            out.set_double("x", pos.curr.x());
            out.set_double("y", pos.curr.y());
            out.set_double("z", pos.curr.z());
            if other.ver() == ProtocolVersion::V1_8 {
              out.set_byte("yaw", (pos.yaw / 360.0 * 256.0).round() as i8 as u8);
              out.set_byte("pitch", (pos.pitch / 360.0 * 256.0).round() as i8 as u8);
            } else {
              out.set_float("yaw", pos.yaw);
              out.set_float("pitch", pos.pitch);
            }
            out.set_byte("flags", 0); // All positions are absolute
          }
        }
        out.set_bool("on_ground", true);
        other.conn().send(out).await;
      }
    }
    if old_chunk != new_chunk {
      let view_distance = 10; // TODO: Listen for client settings on this
      let delta = new_chunk - old_chunk;
      let new_tl = new_chunk - ChunkPos::new(view_distance, view_distance);
      let new_br = new_chunk + ChunkPos::new(view_distance, view_distance);
      let old_tl = old_chunk - ChunkPos::new(view_distance, view_distance);
      let old_br = old_chunk + ChunkPos::new(view_distance, view_distance);
      // Sides (including corners)
      let load_min;
      let load_max;
      let unload_min;
      let unload_max;
      match delta.x().cmp(&0) {
        Ordering::Greater => {
          load_min = ChunkPos::new(old_br.x(), new_tl.z());
          load_max = new_br;
          unload_min = old_tl;
          unload_max = ChunkPos::new(new_tl.x(), old_br.z());
        }
        Ordering::Less => {
          load_min = new_tl;
          load_max = ChunkPos::new(old_tl.x(), new_br.z());
          unload_min = ChunkPos::new(new_br.x(), old_tl.z());
          unload_max = old_br;
        }
        _ => {
          load_min = ChunkPos::new(0, 0);
          load_max = ChunkPos::new(0, 0);
          unload_min = ChunkPos::new(0, 0);
          unload_max = ChunkPos::new(0, 0);
        }
      };
      self.load_chunks(load_min, load_max).await;
      self.unload_chunks(unload_min, unload_max).await;
      // Top/Bottom (excluding corners)
      let load_min;
      let load_max;
      let unload_min;
      let unload_max;
      match delta.z().cmp(&0) {
        Ordering::Greater => {
          load_min = ChunkPos::new(new_tl.x(), old_br.z());
          load_max = ChunkPos::new(cmp::min(new_br.x(), old_br.x()), new_br.z());
          unload_min = ChunkPos::new(cmp::max(new_tl.x(), old_tl.x()), old_tl.z());
          unload_max = ChunkPos::new(old_br.x(), new_tl.z());
        }
        Ordering::Less => {
          load_min = ChunkPos::new(cmp::max(old_tl.x(), new_tl.x()), new_tl.z());
          load_max = ChunkPos::new(new_br.x(), old_tl.z());
          unload_min = ChunkPos::new(old_tl.x(), new_br.z());
          unload_max = ChunkPos::new(cmp::min(old_br.x(), new_br.x()), old_br.z());
        }
        _ => {
          load_min = ChunkPos::new(0, 0);
          load_max = ChunkPos::new(0, 0);
          unload_min = ChunkPos::new(0, 0);
          unload_max = ChunkPos::new(0, 0);
        }
      };
      self.load_chunks(load_min, load_max).await;
      self.unload_chunks(unload_min, unload_max).await;
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
        out.set_int("chunk_x", x);
        out.set_int("chunk_z", z);
        self.conn.send(out).await;
      }
    }
  }

  /// Generates the player's metadata for the given version. This will include
  /// all fields possible about the player. This should only be called when
  /// spawning in a new player.
  pub fn metadata(&self, ver: ProtocolVersion) -> Metadata {
    let mut meta = Metadata::new(ver);
    meta.set_byte(0, 0b11111111).unwrap();
    meta
  }

  /// Returns the player's position. This is only updated once per tick. This
  /// also needs to lock a mutex, so you should not call it very often.
  pub fn pos(&self) -> FPos {
    let pos = self.pos.lock().unwrap();
    pos.curr
  }
  /// Returns the player's position and looking direction. This is only updated
  /// once per tick. This also locks a mutex, so you should not call it very
  /// often.
  pub fn pos_look(&self) -> (FPos, f32, f32) {
    let pos = self.pos.lock().unwrap();
    (pos.curr, pos.pitch, pos.yaw)
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
  pub fn look(&self) -> (f32, f32) {
    let pos = self.pos.lock().unwrap();
    (pos.pitch, pos.yaw)
  }

  /// Returns true if the player is within render distance of the given chunk
  pub fn in_view(&self, pos: ChunkPos) -> bool {
    let delta = pos - self.pos().block().chunk();
    // TODO: Store view distance
    delta.x().abs() <= 10 && delta.z().abs() <= 10
  }
}

#[test]
fn assert_sync() {
  fn is_sync<T: Send + Sync>() {}
  is_sync::<Player>(); // only compiles is player is Sync
}

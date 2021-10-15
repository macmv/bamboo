use rayon::prelude::*;
use std::{
  cmp::Ordering,
  fmt,
  sync::{Arc, Mutex, MutexGuard},
};

use crossbeam_channel::Sender;
use sc_common::{
  math::{ChunkPos, FPos, Pos, PosError},
  net::cb,
  util::{Chat, UUID},
  version::ProtocolVersion,
};

use crate::{
  command::CommandSender,
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
  eid:           i32,
  // Player's username
  username:      String,
  uuid:          UUID,
  tx:            Sender<cb::Packet>,
  ver:           ProtocolVersion,
  world:         Arc<World>,
  view_distance: u32,

  inv: Mutex<PlayerInventory>,

  pos: Mutex<PlayerPosition>,
}

impl fmt::Debug for Player {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Player")
      .field("username", &self.username)
      .field("uuid", &self.uuid)
      .field("ver", &self.ver)
      .field("view_distance", &self.view_distance)
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
    tx: Sender<cb::Packet>,
    ver: ProtocolVersion,
    world: Arc<World>,
    pos: FPos,
  ) -> Self {
    Player {
      eid,
      username,
      uuid,
      tx,
      ver,
      world,
      view_distance: 10,
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

  /// Returns the player's entity id. Used to send packets about entities.
  pub fn eid(&self) -> i32 {
    self.eid
  }
  /// Returns the player's uuid. Used to lookup players in the world.
  pub fn id(&self) -> UUID {
    self.uuid
  }

  /// Returns the version that this client connected with. This will only change
  /// if the player disconnects and logs in with another client.
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
    self
      .conn()
      .send(cb::Packet::Chat {
        message:      msg.to_json(),
        position:     0, // Chat box, not system message or over hotbar
        sender_v1_16: Some(self.id()),
      })
      .await;
  }
  /// Sends the player a chat message, which will appear over their hotbar.
  pub async fn send_hotbar(&self, msg: &Chat) {
    self
      .conn()
      .send(cb::Packet::Chat {
        message:      msg.to_json(),
        position:     2, // Hotbar, not chat box or system message
        sender_v1_16: Some(self.id()),
      })
      .await;
  }
  /// Disconnects the player. The given chat message will be shown on the
  /// loading screen.
  ///
  /// This may not have an effect immediately. This only sends a disconnect
  /// packet. Assuming normal operation, the client will then disconnect after
  /// they have recieved this packet.
  ///
  /// TODO: This should terminate the connection after this packet is sent.
  /// Closing the channel will drop the packet before it can be sent, so we need
  /// some other way of closing it later.
  pub async fn disconnect<C: Into<Chat>>(&self, msg: C) {
    self.send(cb::Packet::KickDisconnect { reason: msg.into().to_json() });
  }

  /// Updates the player's position/velocity. This will apply gravity, and do
  /// collision checks. Should never be called at a different rate than the
  /// global tick rate.
  pub(crate) fn tick(&self) {
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
      for other in self.world.players().iter().in_view(pos.curr.chunk()).not(self.uuid) {
        // Make player move for other
        let yaw;
        let pitch;
        let on_ground = true;
        if look_changed {
          other.send(cb::Packet::EntityHeadRotation {
            entity_id: self.eid,
            head_yaw:  (pos.yaw / 360.0 * 256.0).round() as i8,
          });
          yaw = (pos.yaw / 360.0 * 256.0).round() as i8;
          pitch = (pos.pitch / 360.0 * 256.0).round() as i8;
        } else {
          yaw = 0;
          pitch = 0;
        }
        if pos_changed {
          let mut d_x_v1_8 = 0;
          let mut d_x_v1_9 = 0;
          let mut d_y_v1_8 = 0;
          let mut d_y_v1_9 = 0;
          let mut d_z_v1_8 = 0;
          let mut d_z_v1_9 = 0;
          let mut dx = pos.curr.x() - pos.prev.x();
          let mut dy = pos.curr.y() - pos.prev.y();
          let mut dz = pos.curr.z() - pos.prev.z();
          let abs_pos;
          if other.ver() == ProtocolVersion::V1_8 {
            dx *= 32.0;
            dy *= 32.0;
            dz *= 32.0;
            if dx.abs() > i8::MAX.into() || dy.abs() > i8::MAX.into() || dz.abs() > i8::MAX.into() {
              abs_pos = true;
            } else {
              // As truncates any negative floats to 0, but just copies the bits for i8 -> u8
              d_x_v1_8 = dx.round() as i8;
              d_y_v1_8 = dy.round() as i8;
              d_z_v1_8 = dz.round() as i8;
              abs_pos = false;
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
              abs_pos = true;
            } else {
              d_x_v1_9 = dx.round() as i16;
              d_y_v1_9 = dy.round() as i16;
              d_z_v1_9 = dz.round() as i16;
              abs_pos = false;
            }
          };
          if abs_pos {
            let yaw;
            let pitch;
            if other.ver() == ProtocolVersion::V1_8 {
              yaw = (pos.yaw / 360.0 * 256.0).round() as i8;
              pitch = (pos.pitch / 360.0 * 256.0).round() as i8;
            } else {
              yaw = pos.yaw as i8;
              pitch = pos.pitch as i8;
            }
            // Cannot use relative move
            other.send(cb::Packet::EntityTeleport {
              entity_id: self.eid,
              x_v1_8: Some(pos.curr.fixed_x()),
              x_v1_9: Some(pos.curr.x()),
              y_v1_8: Some(pos.curr.fixed_y()),
              y_v1_9: Some(pos.curr.y()),
              z_v1_8: Some(pos.curr.fixed_z()),
              z_v1_9: Some(pos.curr.z()),
              yaw,
              pitch,
              on_ground,
            });
          } else {
            // Can use relative move, and we know that pos_changed is true
            if look_changed {
              other.send(cb::Packet::EntityMoveLook {
                entity_id: self.eid,
                d_x_v1_8: Some(d_x_v1_8),
                d_x_v1_9: Some(d_x_v1_9),
                d_y_v1_8: Some(d_y_v1_8),
                d_y_v1_9: Some(d_y_v1_9),
                d_z_v1_8: Some(d_z_v1_8),
                d_z_v1_9: Some(d_z_v1_9),
                yaw,
                pitch,
                on_ground,
              });
            } else {
              other.send(cb::Packet::RelEntityMove {
                entity_id: self.eid,
                d_x_v1_8: Some(d_x_v1_8),
                d_x_v1_9: Some(d_x_v1_9),
                d_y_v1_8: Some(d_y_v1_8),
                d_y_v1_9: Some(d_y_v1_9),
                d_z_v1_8: Some(d_z_v1_8),
                d_z_v1_9: Some(d_z_v1_9),
                on_ground,
              });
            }
          }
        } else {
          // Pos changed is false, so look_changed must be true
          other.send(cb::Packet::EntityLook { entity_id: self.eid, yaw, pitch, on_ground });
        }
      }
    }
    if old_chunk != new_chunk {
      let delta = new_chunk - old_chunk;
      let v = self.view_distance as i32;
      let new_max = new_chunk + ChunkPos::new(v, v);
      let new_min = new_chunk - ChunkPos::new(v, v);
      let old_max = old_chunk + ChunkPos::new(v, v);
      let old_min = old_chunk - ChunkPos::new(v, v);
      // Sides (including corners)
      let load_min;
      let load_max;
      let unload_min;
      let unload_max;
      match delta.x().cmp(&0) {
        Ordering::Greater => {
          load_min = ChunkPos::new(old_max.x() + 1, new_min.z());
          load_max = new_max;
          unload_min = old_min;
          unload_max = ChunkPos::new(new_min.x() - 1, old_max.z());
        }
        Ordering::Less => {
          load_min = new_min;
          load_max = ChunkPos::new(old_min.x() - 1, new_max.z());
          unload_min = ChunkPos::new(new_max.x() + 1, old_min.z());
          unload_max = old_max;
        }
        _ => {
          load_min = ChunkPos::new(0, 0);
          load_max = ChunkPos::new(0, 0);
          unload_min = ChunkPos::new(0, 0);
          unload_max = ChunkPos::new(0, 0);
        }
      };
      self.load_chunks(load_min, load_max);
      self.unload_chunks(unload_min, unload_max);
      // Top/Bottom (excluding corners)
      let load_min;
      let load_max;
      let unload_min;
      let unload_max;
      match delta.z().cmp(&0) {
        Ordering::Greater => {
          load_min = ChunkPos::new(new_min.x().max(old_min.x()), old_max.z() + 1);
          load_max = ChunkPos::new(new_max.x().min(old_max.x()), new_max.z());
          unload_min = ChunkPos::new(new_min.x().max(old_min.x()), old_min.z());
          unload_max = ChunkPos::new(new_max.x().min(old_max.x()), new_min.z() - 1);
        }
        Ordering::Less => {
          load_min = ChunkPos::new(new_min.x().max(old_min.x()), new_min.z());
          load_max = ChunkPos::new(new_max.x().min(old_max.x()), old_min.z() - 1);
          unload_min = ChunkPos::new(new_min.x().max(old_min.x()), new_max.z() + 1);
          unload_max = ChunkPos::new(new_max.x().min(old_max.x()), old_max.z());
        }
        _ => {
          load_min = ChunkPos::new(0, 0);
          load_max = ChunkPos::new(0, 0);
          unload_min = ChunkPos::new(0, 0);
          unload_max = ChunkPos::new(0, 0);
        }
      };
      self.load_chunks(load_min, load_max);
      self.unload_chunks(unload_min, unload_max);
    }
  }

  /// Loads the chunks between min and max, inclusive.
  fn load_chunks(&self, min: ChunkPos, max: ChunkPos) {
    if min == max {
      return;
    }
    // Generate the chunks on multiple threads
    let chunks = Mutex::new(vec![]);
    if (min.x() - max.x()).abs() > (min.z() - max.z()).abs() {
      (min.x()..=max.x()).into_par_iter().for_each(|x| {
        for z in min.z()..=max.z() {
          let pos = ChunkPos::new(x, z);
          if self.world.has_loaded_chunk(pos) {
            continue;
          }
          let c = self.world.pre_generate_chunk(pos);
          chunks.lock().unwrap().push((pos, c));
        }
      });
    } else {
      (min.z()..=max.z()).into_par_iter().for_each(|z| {
        for x in min.x()..=max.x() {
          let pos = ChunkPos::new(x, z);
          if self.world.has_loaded_chunk(pos) {
            continue;
          }
          let c = self.world.pre_generate_chunk(pos);
          chunks.lock().unwrap().push((pos, c));
        }
      });
    }
    // Calling store_chunks is a race condition! We check for has_loaded_chunk
    // above, but the chunks could have been changed between that call and now.
    // Calling store_chunks could potentially make us loose data.
    self.world.store_chunks_no_overwrite(chunks.into_inner().unwrap());
    for x in min.x()..=max.x() {
      for z in min.z()..=max.z() {
        self.send(self.world.serialize_chunk(ChunkPos::new(x, z), self.ver().block()));
      }
    }
  }
  fn unload_chunks(&self, min: ChunkPos, max: ChunkPos) {
    if min == max {
      return;
    }
    for x in min.x()..=max.x() {
      for z in min.z()..=max.z() {
        if self.ver() == ProtocolVersion::V1_8 {
          self.send(cb::Packet::MapChunk {
            x:                                     x.into(),
            z:                                     z.into(),
            ground_up:                             true,
            bit_map_v1_8:                          Some(0),
            bit_map_v1_9:                          None,
            chunk_data:                            vec![0], /* Need a length prefix. 0 varint
                                                             * is a single 0 byte */
            biomes_v1_15:                          None,
            biomes_v1_16_2:                        None,
            block_entities_v1_9_4:                 None,
            heightmaps_v1_14:                      None,
            ignore_old_data_v1_16_removed_v1_16_2: None,
          });
        } else {
          self.send(cb::Packet::UnloadChunk { chunk_x_v1_9: Some(x), chunk_z_v1_9: Some(z) });
        }
      }
    }
  }

  /// Generates the player's metadata for the given version. This will include
  /// all fields possible about the player. This should only be called when
  /// spawning in a new player.
  pub fn metadata(&self, ver: ProtocolVersion) -> Metadata {
    let meta = Metadata::new(ver);
    // meta.set_byte(0, 0b00000000).unwrap();
    meta
  }

  /// Returns the player's position. This is only updated once per tick. This
  /// also needs to lock a mutex, so you should not call it very often.
  pub fn pos(&self) -> FPos {
    let pos = self.pos.lock().unwrap();
    pos.curr
  }
  /// Returns the player's block position. This is the block that their feet are
  /// in. This is the same thing as calling [`p.pos().block()`](Self::pos).
  fn block_pos(&self) -> Pos {
    self.pos().block()
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

  /// Sets the player's fly speed. Unlike the packet, this is a multipler. So
  /// setting their flyspeed to 1.0 is the default speed.
  pub async fn set_flyspeed(&self, speed: f32) {
    self.send(cb::Packet::Abilities {
      // 0x01: No damage
      // 0x02: Flying
      // 0x04: Can fly
      // 0x08: Can instant break
      flags:         0x02 | 0x04 | 0x08,
      flying_speed:  speed * 0.05,
      walking_speed: 0.1,
    });
  }

  /// Sends a block update packet for the block at the given position. This
  /// ensures that the client sees what the server sees at that position.
  ///
  /// This is mostly used for placing blocks. If you place a block on a stone
  /// block, then the position you clicked on is not the same as the position
  /// where the new block is. However, if you click on tall grass, then the tall
  /// grass will be replaced by the new block. The client assumes this, and it
  /// ends up becoming desyncronized from the server. So this function is called
  /// on that tall grass block, to prevent the client from showing the wrong
  /// block.
  pub async fn sync_block_at(&self, pos: Pos) -> Result<(), PosError> {
    let ty = self.world().get_block(pos)?;
    self.send(cb::Packet::BlockChange {
      location: pos,
      type_:    self.world().block_converter().to_old(ty.id(), self.ver().block()) as i32,
    });
    Ok(())
  }

  /// Sends the given packet to this player. This will be flushed as soon as the
  /// outgoing buffer as space, which is immediately in most situations. If a
  /// bunch of data is being sent at once, this function can block. So this
  /// technically can result in deadlocks, but the way the threads are setup
  /// right now mean that no channel will block another channel, so in practice
  /// this will only produce slow downs, never deadlocks.
  pub fn send(&self, p: cb::Packet) {
    self.tx.send(p).unwrap();
  }

  /// Returns true if the player's connection is closed.
  pub fn closed(&self) -> bool {
    self.tx.closed()
  }
}

impl CommandSender for Player {
  fn block_pos(&self) -> Option<Pos> {
    Some(self.block_pos())
  }
}

#[test]
fn assert_sync() {
  fn is_sync<T: Send + Sync>() {}
  is_sync::<Player>(); // only compiles if player is Sync
}

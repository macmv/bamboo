use super::{DigProgress, Player, PlayerPosition};
use crate::{block, event, event::EventFlow, math::Vec3};
use bb_common::{
  math::{ChunkPos, Pos},
  net::cb,
  version::ProtocolVersion,
};
use std::{
  cmp::Ordering,
  sync::Arc,
  time::{Duration, Instant},
};

impl DigProgress {
  pub fn new(pos: Pos, kind: block::Kind) -> Self {
    DigProgress { progress: 0.0, pos, kind, wants_finish: false }
  }
}

impl Player {
  /// Updates the player's position/velocity. This will apply gravity, and do
  /// collision checks. Should never be called at a different rate than the
  /// global tick rate.
  pub(crate) fn tick(self: &Arc<Self>) {
    let mut health = self.health.lock();
    let old_chunk;
    let new_chunk;
    let look_changed;
    let pos_changed;
    let needs_set_pos;
    let mut invalid_move = None;
    let pos = {
      let mut pos = self.pos.lock();
      self.update_dig_progress(&mut pos);

      look_changed = pos.yaw != pos.next_yaw || pos.pitch != pos.next_pitch;
      pos_changed = pos.curr != pos.next;

      let mut teleported = false;
      if let Some(tp) = pos.teleport_to {
        if pos.next.dist_squared(tp) < 1.0 {
          pos.prev = pos.curr;
          pos.curr = pos.next;
          pos.teleport_to = None;
          teleported = true;
        }
      }

      if !teleported {
        // This uses the position from the previous tick to find the velocity on that
        // tick, and finds what we expected the client to move on that tick.
        let expected_move = Vec3::from(pos.curr - pos.prev);
        // This uses the new position to find the velocity that the client moved this
        // tick.
        let actual_move = Vec3::from(pos.next - pos.curr);
        // The player's acceleration.
        let accel = expected_move - actual_move;

        // In 1.8, on the client, I never saw this go above 1.5. On the server, this
        // goes above 2 every now and then.
        let flying = self.flying();
        let accel_len = accel.len();
        if (!flying && accel_len > 3.0) || (flying && accel_len > f64::from(self.fly_speed()) * 5.0)
        {
          warn!(
            "{} moved too fast (pos: {} {} {})",
            self.username, pos.curr.x, pos.curr.y, pos.curr.z
          );
          invalid_move = Some(cb::packet::SetPosLook {
            pos:             pos.curr,
            yaw:             pos.yaw,
            pitch:           pos.pitch,
            flags:           0,
            teleport_id:     0,
            should_dismount: false,
          });
          pos.prev = pos.curr;
        } else {
          pos.prev = pos.curr;
          pos.curr = pos.next;
        }
      }
      pos.vel = (pos.curr - pos.prev).into();

      pos.yaw = pos.next_yaw;
      // We want to keep yaw within -180..=180
      pos.yaw %= 360.0;
      if pos.yaw < -180.0 {
        pos.yaw += 360.0;
      }
      if pos.yaw > 180.0 {
        pos.yaw -= 360.0;
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

      let now = Instant::now();
      if pos_changed && now.duration_since(pos.last_set_pos) > Duration::from_secs(5) {
        pos.last_set_pos = now;
        needs_set_pos = true;
      } else {
        needs_set_pos = false;
      }

      // If we teleported, we want out move history to look like we've just been
      // standing here, so that movement checks pass next tick. However, we want the
      // returned `prev` to be the actual previous position, as that is used for
      // sending movement deltas.
      let ret = pos.clone();
      if teleported {
        pos.prev = pos.curr;
      }
      ret
    };

    // We don't want `pos` locked while sending packets
    if let Some(p) = invalid_move {
      self.send(p);
    }
    // Handle edge case for players sending dig finish too early.
    self.check_dig_wants_finish();
    if pos_changed || look_changed {
      for other in self.world.players().iter().in_view(pos.curr.chunk()).not(self.uuid) {
        // Make player move for other
        let yaw;
        let pitch;
        let on_ground = true;
        if look_changed {
          // other.send(cb::packet::EntityHeadRotation {
          //   entity_id: self.eid,
          //   head_yaw:  (pos.yaw / 360.0 * 256.0).round() as i8,
          // });
          yaw = (pos.yaw / 360.0 * 256.0).round() as i8;
          pitch = (pos.pitch / 360.0 * 256.0).round() as i8;
        } else {
          yaw = 0;
          pitch = 0;
        }
        if pos_changed {
          let dx = pos.curr.x() - pos.prev.x();
          let dy = pos.curr.y() - pos.prev.y();
          let dz = pos.curr.z() - pos.prev.z();
          // On 1.8, we send EntityMove packets with the following value: delta * 32. On
          // 1.9+, we send it like so: delta * 4096. On 1.8, we use a single byte for the
          // delta, whereas on 1.9+, we use a short.This means the total blocks you can
          // represent in each of these forms can be calculated like so:
          //
          // 1.8:  (1 << 8) / 32 = 8
          // 1.9+: (1 << 16) / 4096 = 16
          //
          // Because we need to represent negative deltas, we arrive at a total distance
          // of 4 blocks on 1.8, and 8 blocks on 1.9+. That is how we arrive at the
          // conditional below.
          let abs_pos = if other.ver() == ProtocolVersion::V1_8 {
            dx.abs() > 4.0 || dy.abs() > 4.0 || dz.abs() > 4.0
          } else {
            dx.abs() > 8.0 || dy.abs() > 8.0 || dz.abs() > 8.0
          };
          if abs_pos || needs_set_pos {
            // Cannot use relative move
            let yaw;
            let pitch;
            if other.ver() == ProtocolVersion::V1_8 {
              yaw = (pos.yaw / 360.0 * 256.0).round() as i8;
              pitch = (pos.pitch / 360.0 * 256.0).round() as i8;
            } else {
              yaw = pos.yaw as i8;
              pitch = pos.pitch as i8;
            }
            other.send(cb::packet::EntityPos {
              eid: self.eid,
              x: pos.curr.x(),
              y: pos.curr.y(),
              z: pos.curr.z(),
              yaw,
              pitch,
              on_ground,
            });
          } else {
            // Can use relative move, and we know that pos_changed is true
            if look_changed {
              other.send(cb::packet::EntityMoveLook {
                eid: self.eid,
                x: (dx * 4096.0) as i16,
                y: (dy * 4096.0) as i16,
                z: (dz * 4096.0) as i16,
                yaw,
                pitch,
                on_ground,
              });
              other.send(cb::packet::EntityHeadLook { eid: self.eid, yaw });
            } else {
              other.send(cb::packet::EntityMove {
                eid: self.eid,
                x: (dx * 4096.0) as i16,
                y: (dy * 4096.0) as i16,
                z: (dz * 4096.0) as i16,
                on_ground,
              });
            }
          }
        } else {
          // Pos changed is false, so look_changed must be true
          other.send(cb::packet::EntityLook { eid: self.eid, yaw, pitch, on_ground });
          other.send(cb::packet::EntityHeadLook { eid: self.eid, yaw });
        }
      }
    }
    if old_chunk != new_chunk {
      if self.ver() >= ProtocolVersion::V1_14 {
        self.send(cb::packet::UpdateViewPos { pos: new_chunk });
      }
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

    if health.hit_delay > 0 {
      health.hit_delay -= 1;
    }
  }

  /// Loads the chunks between min and max, inclusive.
  fn load_chunks(self: &Arc<Self>, min: ChunkPos, max: ChunkPos) {
    if min == max {
      return;
    }
    // If we have the chunk, send it. Otherwise, queue the chunk to be generated,
    // and send it to `self.conn` once generated.
    //
    // `queue_chunk` will simply send the chunk if it has already been loaded, so
    // there aren't any race conditions in this function.
    for x in min.x()..=max.x() {
      for z in min.z()..=max.z() {
        let pos = ChunkPos::new(x, z);
        if self.world.has_loaded_chunk(pos) {
          self.send_chunk(pos, || self.world.serialize_chunk(pos).into());
        } else {
          self.world.queue_chunk(pos, self);
        }
      }
    }
  }
  /// Sends the chunk to the client, and records that the client knows about
  /// this chunk. This makes sure we avoid ever leaking memory on the client.
  pub(crate) fn send_chunk(&self, pos: ChunkPos, f: impl FnOnce() -> cb::Packet) {
    if self.in_view(pos) {
      let mut lock = self.loaded_chunks.lock();
      if !lock.contains(&pos) {
        lock.insert(pos);
        drop(lock);
        self.world.inc_view(pos);
        self.send(f());
      }
    }
  }
  /// Sends the unload packet for this chunk to the client, and records that the
  /// client no longer has that chunk in memory.
  fn send_unload_chunk(&self, pos: ChunkPos) {
    let mut lock = self.loaded_chunks.lock();
    if lock.remove(&pos) {
      drop(lock);
      self.world.dec_view(pos);
      self.send(cb::packet::UnloadChunk { pos });
    }
  }
  /// Unloads all the chunks that this player can see from the world. This will
  /// call dec_view for all the chunks this player can see. This does not send
  /// any packets! It should only be used internally when a player is being
  /// removed.
  ///
  /// Note that this won't update the `loaded_chunks` table. This is again
  /// because the player is about to be removed, so this table will never be
  /// looked up again.
  pub(crate) fn unload_all(&self) {
    let chunk = {
      let pos = self.pos.lock();
      pos.curr.block().chunk()
    };
    let v = self.view_distance as i32;
    let max = chunk + ChunkPos::new(v, v);
    let min = chunk - ChunkPos::new(v, v);
    for x in min.x()..=max.x() {
      for z in min.z()..=max.z() {
        self.world.dec_view(ChunkPos::new(x, z));
      }
    }
  }
  fn unload_chunks(&self, min: ChunkPos, max: ChunkPos) {
    if min == max {
      return;
    }
    for x in min.x()..=max.x() {
      for z in min.z()..=max.z() {
        let pos = ChunkPos::new(x, z);
        self.world.unqueue_chunk(pos, self);
        self.send_unload_chunk(pos);
      }
    }
  }

  pub(crate) fn start_digging(self: &Arc<Self>, pos: Pos) {
    // Silently ignore dig packets outside the world.
    if let Ok(kind) = self.world.get_kind(pos) {
      let mut ppos = self.pos.lock();
      let speed = self.mining_speed(ppos.curr.block(), kind);
      if speed >= 1.0 {
        // Insta-break the block
        drop(ppos); // drop the player position lock, so this can clear ppos.dig_progress
        if self.block_break_event(pos).is_handled() {
          self.sync_block_at(pos);
        } else {
          let _ = self.world.break_block(pos);
        }
      } else {
        let mut progress = DigProgress::new(pos, kind);
        // The client does this twice on the first tick, as they need to get mining
        // speed to check for insta-break.
        progress.progress += speed;
        ppos.dig_progress = Some(progress);
      }
    }
  }
  pub(crate) fn cancel_digging(&self) { self.pos.lock().dig_progress = None; }
  pub(crate) fn finish_digging(self: &Arc<Player>, pos: Pos) {
    let mut sync = false;
    let mut finished = false;
    {
      let mut lock = self.pos.lock();
      if let Some(p) = &mut lock.dig_progress {
        if pos == p.pos {
          if p.progress >= 1.0 {
            finished = true;
          } else {
            p.wants_finish = true;
          }
        } else {
          // If we get a different position, reset digging progress, and send a sync back.
          // This is most likely someone trying to cheat.
          sync = true;
        }
      } else {
        sync = true;
      }
      if finished || sync {
        lock.dig_progress = None;
      }
    }
    if finished {
      if self.block_break_event(pos).is_continue() {
        if !self.world().break_block(pos).unwrap() {
          self.sync_block_at(pos);
        }
      } else {
        self.sync_block_at(pos);
      }
    } else if sync {
      self.sync_block_at(pos);
    }
  }
  pub(crate) fn block_break_event(self: &Arc<Player>, pos: Pos) -> EventFlow {
    self.world().events().player_request(event::BlockBreak {
      player: self.clone(),
      pos,
      block: self.world().get_block(pos).unwrap().ty().to_store(),
    })
  }

  fn mining_speed(&self, curr_pos: Pos, kind: block::Kind) -> f64 {
    // Handles block/item type, and efficiency levels
    let mut speed = self
      .lock_inventory()
      .main_hand()
      .mining_speed(self.world.world_manager().block_converter().get(kind));

    // TODO: Multiply by haste:
    // if self.has_haste() {
    //   speed *= 1.0 + (haste_level + 1) * 0.2;
    // }

    // TODO: Multiply by mining fatigue:
    // match mining_fatigue {
    //   0 => speed * 0.3,
    //   1 => speed * 0.09,
    //   2 => speed * 0.0027,
    //   _ => speed * 0.00081,
    // }

    // If we are standing in water, digging is 5 times slower.
    if self.world.get_kind(curr_pos) == Ok(block::Kind::Water) {
      speed *= 0.2;
    }

    // TODO: Multiply by on ground:
    // if pos.on_ground {
    //   speed *= 0.2;
    // }
    // We can't really trust the client for this, as they can say they're on
    // ground at any point. This means we need to do a bit of physics on the
    // player to figure out if they are on ground.
    speed
  }

  fn update_dig_progress(&self, pos: &mut PlayerPosition) {
    if let Some(p) = &mut pos.dig_progress {
      p.progress += self.mining_speed(pos.curr.block(), p.kind);
    }
  }

  fn check_dig_wants_finish(self: &Arc<Self>) {
    let mut finish = None;
    {
      let mut lock = self.pos.lock();
      if let Some(p) = &lock.dig_progress {
        if p.wants_finish && p.progress >= 1.0 {
          finish = Some(p.pos);
        }
      }
      if finish.is_some() {
        lock.dig_progress = None;
      }
    }
    if let Some(pos) = finish {
      if self
        .world()
        .events()
        .player_request(event::BlockBreak {
          player: self.clone(),
          pos,
          block: self.world().get_block(pos).unwrap().ty().to_store(),
        })
        .is_continue()
      {
        if !self.world().break_block(pos).unwrap() {
          self.sync_block_at(pos);
        }
      } else {
        self.sync_block_at(pos);
      }
    }
  }
}

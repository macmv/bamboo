use super::Player;
use parking_lot::Mutex;
use rayon::prelude::*;
use sc_common::{math::ChunkPos, net::cb, version::ProtocolVersion};
use std::cmp::Ordering;

impl Player {
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
      // We want to keep yaw within -180..=180
      pos.yaw = pos.yaw % 360.0;
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
          chunks.lock().push((pos, c));
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
          chunks.lock().push((pos, c));
        }
      });
    }
    // Calling store_chunks is a race condition! We check for has_loaded_chunk
    // above, but the chunks could have been changed between that call and now.
    // Calling store_chunks could potentially make us loose data.
    self.world.store_chunks_no_overwrite(chunks.into_inner());
    for x in min.x()..=max.x() {
      for z in min.z()..=max.z() {
        let pos = ChunkPos::new(x, z);
        self.world.inc_view(pos);
        self.send(self.world.serialize_chunk(pos, self.ver().block()));
      }
    }
  }
  /// Unloads all the chunks that this player can see from the world. This will
  /// call dec_view for all the chunks this player can see. This does not send
  /// any packets! It should only be used internally when a player is being
  /// removed.
  pub(crate) fn unload_all(&self) {
    let chunk = {
      let pos = self.pos.lock().unwrap();
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
        self.world.dec_view(ChunkPos::new(x, z));
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
}
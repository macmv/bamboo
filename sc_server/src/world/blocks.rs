use crate::{block, net, world::World};
use sc_common::{
  math::{ChunkPos, Pos, PosError},
  net::cb,
};
use std::{cmp::Ordering, collections::HashMap};

/// General block manipulation functions
impl World {
  pub fn get_block(&self, pos: Pos) -> Result<block::Type, PosError> {
    self.chunk(pos.chunk(), |c| c.get_type(pos.chunk_rel()))
  }
  /// This sets a block within the world. It will return an error if the
  /// position is outside of the world. Unlike
  /// [`MultiChunk::set_type`](chunk::MultiChunk::set_type), this will send
  /// packets to anyone within render distance of the given chunk.
  pub fn set_block(&self, pos: Pos, ty: block::Type) -> Result<(), PosError> {
    self.chunk(pos.chunk(), |mut c| c.set_type(pos.chunk_rel(), ty))?;

    for p in self.players().iter().in_view(pos.chunk()) {
      p.send(cb::Packet::BlockChange {
        location: pos,
        type_:    self.block_converter.to_old(ty.id(), p.ver().block()) as i32,
      });
    }
    Ok(())
  }

  /// This sets a block within the world. This will use the default type of the
  /// given kind. It will return an error if the position is outside of the
  /// world.
  pub fn set_kind(&self, pos: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.set_block(pos, self.block_converter.get(kind).default_type())
  }

  /// Fills the given region with the given block type. Min must be less than or
  /// equal to max. Use [`min_max`](Pos::min_max) to convert two corners of a
  /// cube into a min and max.
  pub fn fill_rect(&self, min: Pos, max: Pos, ty: block::Type) -> Result<(), PosError> {
    // Small fills should just send a block update, instead of a multi block change.
    if min == max {
      return self.set_block(min, ty);
    }
    for x in min.chunk_x()..=max.chunk_x() {
      for z in min.chunk_z()..=max.chunk_z() {
        let pos = ChunkPos::new(x, z);

        let min_x = if min.chunk_x() == x { min.chunk_rel_x() } else { 0 };
        let min_z = if min.chunk_z() == z { min.chunk_rel_z() } else { 0 };
        let max_x = if max.chunk_x() == x { max.chunk_rel_x() } else { 15 };
        let max_z = if max.chunk_z() == z { max.chunk_rel_z() } else { 15 };

        let min = Pos::new(min_x, min.y, min_z);
        let max = Pos::new(max_x, max.y, max_z);

        self.chunk(pos, |mut c| c.fill(min, max, ty))?;

        let num_blocks_changed = min.to(max).len();
        // 2048 block is where chunk data packets are smaller. Multi block change
        // packets use varints, so this is not an exact value, but it would be ideal
        // (for packet size) to just compare with 2048 here.
        //
        // However, the minecraft client is terrible, and does things very slowly. So
        // any time there is a large multi block change, the client will freeze up. That
        // is why this check is against such a low number.
        if num_blocks_changed > 128 {
          // Map of block version to packets. This server is optimized at many players
          // being online, so we only generate the chunk once for each versions.
          let mut serialized_chunks = HashMap::new();
          for p in self.players().iter().in_view(pos) {
            p.send(
              serialized_chunks
                .entry(pos)
                .or_insert_with(|| {
                  self.serialize_partial_chunk(
                    pos,
                    p.ver().block(),
                    min.chunk_y() as u32,
                    max.chunk_y() as u32,
                  )
                })
                .clone(),
            );
          }
        } else {
          // Map of block versions to multi block change records.
          let mut versions = HashMap::new();
          for p in self.players().iter().in_view(pos) {
            p.send(
              versions
                .entry(p.ver().block())
                .or_insert_with(|| {
                  net::serialize::serialize_multi_block_change(
                    pos,
                    p.ver().block(),
                    min.to(max).map(|pos| {
                      (
                        pos.chunk_rel(),
                        self.block_converter.to_old(ty.id(), p.ver().block()) as i32,
                      )
                    }),
                  )
                })
                .clone(),
            );
          }
        }
      }
    }

    Ok(())
  }

  /// Fills the given region with the default type for the block kind. Min must
  /// be less than or equal to max. Use [`min_max`](Pos::min_max) to convert two
  /// corners of a cube into a min and max.
  pub fn fill_rect_kind(&self, min: Pos, max: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.fill_rect(min, max, self.block_converter.get(kind).default_type())
  }

  /// Fills a flat circle. The center will be the middle of the circle. The
  /// radius is how far the circle extends from the center. The center will act
  /// like it is at (0.5, 0.5, 0.5) within the block. So the circle should not
  /// be offset from the center at all.
  pub fn fill_circle(&self, center: Pos, radius: f32, ty: block::Type) -> Result<(), PosError> {
    // Small circles case. We would run into issues with the corner check if all the
    // corners are outside the circle (and the circle is inside the chunk).
    if radius < 16.0 {
      for z in -radius as i32..=radius as i32 {
        let width = (radius.powi(2) - z.pow(2) as f32).sqrt() as i32;
        self.fill_rect(center + Pos::new(-width, 0, z), center + Pos::new(width, 0, z), ty)?;
      }
      return Ok(());
    }

    // This implementation of filling a circle has two sections. The first section
    // is a loop through all the chunks this circle may touch. Depending on the
    // corners of the chunk, it will then do one of three things:
    //
    // - Empty chunks: This is where the circle covers nothing in this chunk. These
    //   are skipped.
    // - Full chunks: This is where the circle is inside all corners of the chunk.
    //   These are filled with a single `fill_rect` call.
    // - Partial chunks: This is where the circle covers some (not all) of the
    //   corners of the chunk. These are iterated through by row, and each row is
    //   then filled with a `fill_rect` call.
    //
    //   //---------\\
    // //             \\ <- partial chunk
    // |               |
    // |  full chunks  |
    // |               |
    // \\             //
    //   \\---------//     <- empty chunk

    let radius_squared = radius.powi(2);

    let min = center - Pos::new(radius as i32, 0, radius as i32);
    let max = center + Pos::new(radius as i32, 0, radius as i32);

    for chunk_x in min.chunk_x()..=max.chunk_x() {
      for chunk_z in min.chunk_z()..=max.chunk_z() {
        let min = Pos::new(chunk_x * 16, center.y, chunk_z * 16);
        let max = Pos::new(chunk_x * 16 + 15, center.y, chunk_z * 16 + 15);
        let mut corners = 0;
        if (min.dist_squared(center) as f32) < radius_squared {
          corners += 1;
        };
        if (min.with_x(max.x).dist_squared(center) as f32) < radius_squared {
          corners += 1;
        };
        if (min.with_z(max.z).dist_squared(center) as f32) < radius_squared {
          corners += 1;
        };
        if (max.dist_squared(center) as f32) < radius_squared {
          corners += 1;
        };

        match corners {
          // Empty case
          0 => {}
          // Full case
          4 => self.fill_rect(min, max, ty)?,
          // Partial case
          _ => {
            for z in min.z..=max.z {
              let width = (radius.powi(2) - (center.z - z).pow(2) as f32).sqrt() as i32;
              let min_x = match Pos::new(center.x - width, 0, 0).chunk_x().cmp(&chunk_x) {
                Ordering::Less => min.x,
                Ordering::Greater => continue,
                Ordering::Equal => center.x - width,
              };
              let max_x = match Pos::new(center.x + width, 0, 0).chunk_x().cmp(&chunk_x) {
                Ordering::Less => continue,
                Ordering::Greater => max.x,
                Ordering::Equal => center.x + width,
              };
              self.fill_rect(Pos::new(min_x, center.y, z), Pos::new(max_x, center.y, z), ty)?;
            }
          }
        }
      }
    }

    Ok(())
  }

  /// Fills the given circle with the default type for the block kind.
  pub fn fill_circle_kind(
    &self,
    center: Pos,
    radius: f32,
    kind: block::Kind,
  ) -> Result<(), PosError> {
    self.fill_circle(center, radius, self.block_converter.get(kind).default_type())
  }

  /// Fills a sphere. The center will be the middle of this sphere. The radius
  /// is how far the sphere's edge extends from the center. The center will act
  /// like it is at (0.5, 0.5, 0.5) within the block. So the circle should not
  /// be offset from the center at all.
  pub fn fill_sphere(&self, center: Pos, radius: f32, ty: block::Type) -> Result<(), PosError> {
    // Small spheres case. We would run into issues with the corner check if all the
    // corners are outside the circle (and the circle is inside the chunk).
    if radius < 16.0 {
      for y in -radius as i32..=radius as i32 {
        for z in -radius as i32..=radius as i32 {
          let v = radius.powi(2) - (y.pow(2) + z.pow(2)) as f32;
          // Check for this row containing no blocks
          let width = if v < 0.0 { continue } else { v.sqrt() as i32 };
          self.fill_rect(center + Pos::new(-width, y, z), center + Pos::new(width, y, z), ty)?;
        }
      }
      return Ok(());
    }

    // This is the same filling strategy as `fill_circle`, but in 3D. Instead of 4
    // corners, we have 8. We still only have 3 options (full, partial, and empty
    // chunk sections).

    let radius_squared = radius.powi(2);

    let min = center - Pos::new(radius as i32, radius as i32, radius as i32);
    let max = center + Pos::new(radius as i32, radius as i32, radius as i32);

    for chunk_y in min.chunk_y()..=max.chunk_y() {
      for chunk_x in min.chunk_x()..=max.chunk_x() {
        for chunk_z in min.chunk_z()..=max.chunk_z() {
          let min = Pos::new(chunk_x * 16, chunk_y * 16, chunk_z * 16);
          let max = Pos::new(chunk_x * 16 + 15, chunk_y * 16 + 15, chunk_z * 16 + 15);
          let mut corners = 0;
          if (min.dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };
          if (min.with_x(max.x).dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };
          if (min.with_y(max.y).dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };
          if (min.with_z(max.z).dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };
          if (max.with_x(min.x).dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };
          if (max.with_y(min.y).dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };
          if (max.with_z(min.z).dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };
          if (max.dist_squared(center) as f32) < radius_squared {
            corners += 1;
          };

          match corners {
            // Empty case
            0 => {}
            // Full case
            8 => self.fill_rect(min, max, ty)?,
            // Partial case
            _ => {
              for y in min.y..=max.y {
                for z in min.z..=max.z {
                  let v = radius.powi(2) - ((center.y - y).pow(2) + (center.z - z).pow(2)) as f32;
                  // Check for this row containing now blocks.
                  let width = if v < 0.0 { continue } else { v.sqrt() as i32 };
                  let min_x = match Pos::new(center.x - width, 0, 0).chunk_x().cmp(&chunk_x) {
                    Ordering::Less => min.x,
                    Ordering::Greater => continue,
                    Ordering::Equal => center.x - width,
                  };
                  let max_x = match Pos::new(center.x + width, 0, 0).chunk_x().cmp(&chunk_x) {
                    Ordering::Less => continue,
                    Ordering::Greater => max.x,
                    Ordering::Equal => center.x + width,
                  };
                  self.fill_rect(Pos::new(min_x, y, z), Pos::new(max_x, y, z), ty)?;
                }
              }
            }
          }
        }
      }
    }

    Ok(())
  }
  /// Fills the given sphere with the default type for the block kind.
  pub fn fill_sphere_kind(
    &self,
    center: Pos,
    radius: f32,
    kind: block::Kind,
  ) -> Result<(), PosError> {
    self.fill_sphere(center, radius, self.block_converter.get(kind).default_type())
  }

  /// Validates a given block position.
  pub fn check_pos(&self, pos: Pos) -> Result<Pos, PosError> {
    if pos.y < 0 || pos.y >= 256 {
      Err(PosError { pos, msg: "outside of world".into() })
    } else {
      Ok(pos)
    }
  }
}

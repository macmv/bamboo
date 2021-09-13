use crate::{block, net, world::World};
use sc_common::{
  math::{ChunkPos, Pos, PosError},
  net::cb,
};
use std::collections::HashMap;

/// General block manipulation functions
impl World {
  /// This sets a block within the world. It will return an error if the
  /// position is outside of the world. Unlike
  /// [`MultiChunk::set_type`](chunk::MultiChunk::set_type), this will send
  /// packets to anyone within render distance of the given chunk.
  pub async fn set_block(&self, pos: Pos, ty: block::Type) -> Result<(), PosError> {
    self.chunk(pos.chunk(), |mut c| c.set_type(pos.chunk_rel(), ty))?;

    for p in self.players().await.iter().in_view(pos.chunk()) {
      p.conn()
        .send(cb::Packet::BlockChange {
          location: pos,
          type_:    self.block_converter.to_old(ty.id(), p.ver().block()) as i32,
        })
        .await;
    }
    Ok(())
  }

  /// This sets a block within the world. This will use the default type of the
  /// given kind. It will return an error if the position is outside of the
  /// world.
  pub async fn set_kind(&self, pos: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.set_block(pos, self.block_converter.get(kind).default_type()).await
  }

  /// Fills the given region with the given block type. Min must be less than or
  /// equal to max. Use [`min_max`](Pos::min_max) to convert two corners of a
  /// cube into a min and max.
  pub async fn fill_rect(&self, min: Pos, max: Pos, ty: block::Type) -> Result<(), PosError> {
    // Small fills should just send a block update, instead of a multi block change.
    if min == max {
      return self.set_block(min, ty).await;
    }
    let mut blocks_changed = HashMap::new();
    for x in min.chunk_x()..=max.chunk_x() {
      for z in min.chunk_z()..=max.chunk_z() {
        let mut min_x = 0;
        let mut min_z = 0;
        if min.chunk_x() == x {
          min_x = min.chunk_rel_x();
        }
        if min.chunk_z() == z {
          min_z = min.chunk_rel_z();
        }
        let mut max_x = 15;
        let mut max_z = 15;
        if max.chunk_x() == x {
          max_x = max.chunk_rel_x();
        }
        if max.chunk_z() == z {
          max_z = max.chunk_rel_z();
        }
        let min = Pos::new(min_x, min.y, min_z);
        let max = Pos::new(max_x, max.y, max_z);
        blocks_changed.insert(ChunkPos::new(x, z), (min, max));

        self.chunk(ChunkPos::new(x, z), |mut c| c.fill(min, max, ty))?;
      }
    }

    for x in min.chunk_x()..=max.chunk_x() {
      for z in min.chunk_z()..=max.chunk_z() {
        let pos = ChunkPos::new(x, z);
        let (min, max) = &blocks_changed[&pos];
        let num_blocks_changed = (max.x - min.x + 1) * (max.y - min.y + 1) * (max.z - min.z + 1);
        // If we changed more than half the blocks in the chunk, we just resend
        // everything. This is fastsest on average. Multi block changes use varints, so
        // calculating which packet would be smaller is a pain.
        if num_blocks_changed > 2048 {
          // Map of block version to packets. This server is optimized at many players
          // being online, so we only generate the chunk once for each versions.
          let mut serialized_chunks = HashMap::new();
          for p in self.players().await.iter().in_view(pos) {
            p.conn()
              .send(
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
              )
              .await;
          }
        } else {
          // Map of block versions to multi block change records.
          let mut versions = HashMap::new();
          for p in self.players().await.iter().in_view(pos) {
            p.conn()
              .send(
                versions
                  .entry(p.ver().block())
                  .or_insert_with(|| {
                    net::serialize::serialize_multi_block_change(
                      pos,
                      p.ver().block(),
                      min.to(*max).map(|pos| {
                        (
                          pos.chunk_rel(),
                          self.block_converter.to_old(ty.id(), p.ver().block()) as i32,
                        )
                      }),
                    )
                  })
                  .clone(),
              )
              .await;
          }
        }
      }
    }

    Ok(())
  }

  /// Fills the given region with the default type for the block kind. Min must
  /// be less than or equal to max. Use [`min_max`](Pos::min_max) to convert two
  /// corners of a cube into a min and max.
  pub async fn fill_rect_kind(
    &self,
    min: Pos,
    max: Pos,
    kind: block::Kind,
  ) -> Result<(), PosError> {
    self.fill_rect(min, max, self.block_converter.get(kind).default_type()).await
  }
}

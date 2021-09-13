use crate::{block, net, world::World};
use sc_common::{
  math::{ChunkPos, Pos, PosError},
  net::cb,
};
use std::{collections::HashMap, f32::consts::PI};

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
                      min.to(max).map(|pos| {
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

  /// Fills a flat circle. The center will be the middle of the circle. The
  /// radius is how far the circle extends from the center. The center will act
  /// like it is at (0.5, 0.5, 0.5) within the block. So the circle should not
  /// be offset from the center at all.
  pub async fn fill_circle(
    &self,
    center: Pos,
    radius: f32,
    ty: block::Type,
  ) -> Result<(), PosError> {
    // This is a naive implementation. It could not be bothered to integrate these
    // circle functions, so I went with an algorithm like so:
    //
    //     //-------\\
    //   //   edge    \\
    // // |-----------| \\
    // |  | main_rect |  | <- edge
    // |  |           |  |
    // \\ |-----------| //
    //   \\   edge    //
    //     \\-------//
    //
    // The main rect is the largest rectangle that will fit in this circle. It is
    // filled at the start. All the edges are then filled row-by-row, going from the
    // center to the outside.
    let main_rect = ((PI / 4.0).cos() * radius) as i32;
    self
      .fill_rect(
        center - Pos::new(main_rect, 0, main_rect),
        center + Pos::new(main_rect, 0, main_rect),
        ty,
      )
      .await?;
    // Edges. This is off by one because fills are always inclusive.
    for y in main_rect + 1..radius as i32 {
      let start = (radius.powi(2) - y.pow(2) as f32).sqrt() as i32;
      // Top, bottom, right, and left
      self.fill_rect(center + Pos::new(-start, 0, y), center + Pos::new(start, 0, y), ty).await?;
      self.fill_rect(center + Pos::new(-start, 0, -y), center + Pos::new(start, 0, -y), ty).await?;
      self.fill_rect(center + Pos::new(y, 0, -start), center + Pos::new(y, 0, start), ty).await?;
      self.fill_rect(center + Pos::new(-y, 0, -start), center + Pos::new(-y, 0, start), ty).await?;
    }
    Ok(())
  }

  /// Fills the given circle with the default type for the block kind.
  pub async fn fill_circle_kind(
    &self,
    center: Pos,
    radius: f32,
    kind: block::Kind,
  ) -> Result<(), PosError> {
    self.fill_circle(center, radius, self.block_converter.get(kind).default_type()).await
  }
}

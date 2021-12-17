use super::{super::BiomeGen, WorldGen};
use crate::{block, world::chunk::MultiChunk};
use sc_common::math::{ChunkPos, Pos};
use std::collections::HashMap;

pub struct Gen {
  id: usize,
}
impl BiomeGen for Gen {
  fn new(id: usize) -> Gen { Gen { id } }
  fn id(&self) -> usize { self.id }
  fn decorate(
    &self,
    world: &WorldGen,
    chunk_pos: ChunkPos,
    c: &mut MultiChunk,
    tops: &HashMap<Pos, i32>,
  ) {
    for mut p in chunk_pos.columns() {
      if world.is_biome(self, p) {
        let height = tops[&p.chunk_rel()];
        p = p.with_y(height + 1);
        if world.chance(p, 0.30) {
          c.set_kind(p.chunk_rel(), block::Kind::Grass).unwrap();
        }
      }
    }
  }
}

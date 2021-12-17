use super::{super::BiomeGen, WorldGen};
use crate::{block, world::chunk::MultiChunk};
use sc_common::math::ChunkPos;

pub struct Gen {
  id: usize,
}
impl BiomeGen for Gen {
  fn new(id: usize) -> Gen { Gen { id } }
  fn id(&self) -> usize { self.id }
  fn decorate(&self, world: &WorldGen, chunk_pos: ChunkPos, c: &mut MultiChunk) {
    for mut p in chunk_pos.columns() {
      if world.is_biome(self, p) {
        let height = self.height_at(world, p);
        p = p.with_y(height + 1);
        if world.chance(p, 0.30) {
          c.set_kind(p.chunk_rel(), block::Kind::Grass).unwrap();
        }
      }
    }
  }
}

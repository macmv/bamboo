use super::{BiomeGen, WorldGen};
use crate::{block, world::chunk::MultiChunk};
use common::math::{ChunkPos, PointGrid, Pos};

pub struct Gen {
  id:    usize,
  trees: PointGrid,
}

impl BiomeGen for Gen {
  fn new(id: usize) -> Gen {
    Gen { id, trees: PointGrid::new(12345, 16, 5) }
  }
  fn id(&self) -> usize {
    self.id
  }
  fn decorate(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for x in 0..16 {
      for z in 0..16 {
        let p = pos.block() + Pos::new(x, 0, z);
        if world.is_biome(self, p) {
          let height = self.height_at(world, p);
          if self.trees.contains(p.x(), p.z()) {
            c.fill_kind(
              Pos::new(x, height + 1, z),
              Pos::new(x, height + 4, z),
              block::Kind::OakLog,
            )
            .unwrap();
          }
        }
      }
    }
  }
}

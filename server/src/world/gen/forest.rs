use super::{BiomeGen, WorldGen};
use crate::{block, world::chunk::MultiChunk};
use common::math::{ChunkPos, PointGrid, Pos};

pub struct Gen {
  trees: PointGrid,
}

impl Gen {
  pub fn new() -> Box<dyn BiomeGen + Send> {
    Box::new(Self { trees: PointGrid::new(12345, 16, 5) })
  }
}

impl BiomeGen for Gen {
  fn decorate(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for x in 0..16 {
      for z in 0..16 {
        let p = pos.block() + Pos::new(x, 0, z);
        let height = self.height_at(world, p);
        if self.trees.contains(p.x(), p.z()) {
          c.fill_kind(Pos::new(x, height + 1, z), Pos::new(x, height + 4, z), block::Kind::Stone)
            .unwrap();
        }
      }
    }
  }
}

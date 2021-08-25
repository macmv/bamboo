use super::super::WorldGen;
use crate::{block, world::chunk::MultiChunk};
use common::math::{
  terrain::{Point, PointGrid},
  ChunkPos, Pos,
};

pub struct CaveGen {
  origins: PointGrid,
}

impl CaveGen {
  pub fn new(seed: u64) -> Self {
    CaveGen { origins: PointGrid::new(seed, 256, 64) }
  }

  pub fn carve(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for p in pos.columns() {
      if self.origins.contains(Point::new(p.x(), p.z())) {
        for y in 0..80 {
          c.set_kind(p.with_y(y).chunk_rel(), block::Kind::RedWool).unwrap();
        }
      }
    }
  }
}

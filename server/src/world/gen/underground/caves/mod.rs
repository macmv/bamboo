use super::super::WorldGen;
use crate::{block, world::chunk::MultiChunk};
use common::math::{
  terrain::{Point, PointGrid},
  ChunkPos, Pos,
};

mod tree;
use tree::CaveTree;

pub struct CaveGen {
  seed:    u64,
  origins: PointGrid,
}

impl CaveGen {
  pub fn new(seed: u64) -> Self {
    CaveGen { seed, origins: PointGrid::new(seed, 256, 64) }
  }

  pub fn carve(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for p in pos.columns() {
      if self.origins.contains(Point::new(p.x(), p.z())) {
        for y in 0..80 {
          c.set_kind(p.with_y(y).chunk_rel(), block::Kind::RedWool).unwrap();
        }
      }
      for origin in self.origins.neighbors(Point::new(p.x(), p.z()), 3) {
        self.carve_cave(origin, pos, c);
      }
    }
  }

  fn carve_cave(&self, origin: Point, pos: ChunkPos, c: &mut MultiChunk) {
    let tree = CaveTree::new(self.seed ^ ((origin.x as u64) << 32) ^ origin.y as u64);
    for line in tree.lines() {
      for pos in line.traverse() {
        let p = Pos::new(pos.x() + origin.x, pos.y(), pos.z() + origin.y);
        c.set_kind(p.chunk_rel(), block::Kind::BlueWool).unwrap();
      }
    }
  }
}

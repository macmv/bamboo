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
      for origin in self.origins.neighbors(Point::new(p.x(), p.z()), 1) {
        self.carve_cave(origin, pos, c);
      }
    }
  }

  fn carve_cave(&self, origin: Point, chunk_pos: ChunkPos, c: &mut MultiChunk) {
    let tree = CaveTree::new(self.seed ^ ((origin.x as u64) << 32) ^ origin.y as u64);
    for line in tree.lines() {
      for p in line.traverse(origin, chunk_pos) {
        if p.chunk() == chunk_pos {
          let mut min = p.chunk_rel() - Pos::new(1, 1, 1);
          let mut max = p.chunk_rel() + Pos::new(1, 1, 1);
          if min.x() < 0 {
            min = min.with_x(0);
          }
          if min.y() < 0 {
            min = min.with_y(0);
          }
          if min.z() < 0 {
            min = min.with_z(0);
          }
          if max.x() > 15 {
            max = max.with_x(15);
          }
          if max.y() < 0 {
            max = max.with_y(0);
          }
          if max.z() > 15 {
            max = max.with_z(15);
          }
          c.fill_kind(min, max, block::Kind::Air).unwrap();
        }
      }
    }
  }
}

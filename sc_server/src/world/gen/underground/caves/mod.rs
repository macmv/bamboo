use super::super::{
  math::{Point, PointGrid},
  util::Cache,
  WorldGen,
};
use crate::{util::Threaded, world::chunk::MultiChunk};
use sc_common::math::{ChunkPos, Pos};

mod worm;
pub use worm::CaveWorm;

pub struct CaveGen {
  origins: PointGrid,
  worms:   Threaded<Cache<Point, CaveWorm>>,
}

impl CaveGen {
  pub fn new(seed: u64) -> Self {
    CaveGen {
      origins: PointGrid::new(seed, 256, 64),
      worms:   Threaded::new(move || {
        Cache::new(move |origin: Point| {
          let mut worm = CaveWorm::new(
            seed ^ ((origin.x as u64) << 32) ^ origin.y as u64,
            Pos::new(origin.x, 60, origin.y),
          );
          worm.carve(0);
          worm
        })
      }),
    }
  }

  pub fn carve(&self, _world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for origin in self.origins.neighbors(Point::new(pos.block_x(), pos.block_z()), 1) {
      self.carve_cave_worm(origin, pos, c);
      // self.carve_cave_tree(origin, pos, c);
    }
  }

  fn carve_cave_worm(&self, origin: Point, chunk_pos: ChunkPos, c: &mut MultiChunk) {
    self.worms.get(|cache| {
      let worm = cache.get(origin);
      worm.process(chunk_pos, c);
    });
  }
}

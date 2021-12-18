use super::super::{
  math::{Point, PointGrid},
  util::Cache,
  WorldGen,
};
use crate::{block, util::Threaded, world::chunk::MultiChunk};
use noise::{BasicMulti, NoiseFn};
use sc_common::math::{ChunkPos, Pos};

mod worm;
pub use worm::CaveWorm;

pub struct CaveGen {
  origins: PointGrid,
  worms:   Threaded<Cache<Point, CaveWorm>>,
  noise:   BasicMulti,
}

impl CaveGen {
  pub fn new(seed: u64) -> Self {
    let mut noise = BasicMulti::new();
    noise.octaves = 3;
    CaveGen {
      origins: PointGrid::new(seed, 256, 64),
      worms: Threaded::new(move || {
        Cache::new(move |origin: Point| {
          let mut worm = CaveWorm::new(
            seed ^ ((origin.x as u64) << 32) ^ origin.y as u64,
            Pos::new(origin.x, 60, origin.y),
          );
          worm.carve(0);
          worm
        })
      }),
      noise,
    }
  }

  pub fn carve(&self, _world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    self.carve_cave_noise(pos, c);
    for origin in self.origins.neighbors(Point::new(pos.block_x(), pos.block_z()), 1) {
      self.carve_cave_worm(origin, pos, c);
      // self.carve_cave_tree(origin, pos, c);
    }
  }

  fn carve_cave_noise(&self, pos: ChunkPos, c: &mut MultiChunk) {
    let div = 32.0;
    let min_height = 0.0_f64;
    let max_height = 32.0_f64;
    let b_min_height = 0;
    let b_max_height = 48;

    for p in pos.columns() {
      for y in b_min_height..=b_max_height {
        let rel = p.chunk_rel().with_y(y);
        let val = {
          let x = p.x() as f64 / div;
          let y = y as f64 / div / 2.0;
          let z = p.z() as f64 / div;
          self.noise.get([x, y, z])
        };
        let a = (y as f64 - min_height) / (max_height - min_height);
        let b = if y > 20 { 0.0 } else { (15 - y) as f64 / 20.0 };
        let v = a + b;
        let min = v * 0.2 - 0.1;
        if val > min {
          c.set_kind(rel, block::Kind::Air).unwrap();
        }
      }
    }
  }

  fn carve_cave_worm(&self, origin: Point, chunk_pos: ChunkPos, c: &mut MultiChunk) {
    self.worms.get(|cache| {
      let worm = cache.get(origin);
      worm.process(chunk_pos, c);
    });
  }
}

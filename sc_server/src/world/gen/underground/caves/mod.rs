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
  middle:  BasicMulti,
  offset:  BasicMulti,
}

impl CaveGen {
  pub fn new(seed: u64) -> Self {
    let mut noise = BasicMulti::new();
    noise.octaves = 5;
    let mut middle = BasicMulti::new();
    middle.octaves = 3;
    let mut offset = BasicMulti::new();
    offset.octaves = 1;
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
      middle,
      offset,
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
    let min_height = 0.0_f64;
    let max_height = 32.0_f64;
    let b_min_height = 0;
    let b_max_height = 48;

    for p in pos.columns() {
      let middle = (self.middle.get([p.x() as f64, p.z() as f64]) + 1.0) / 2.0 * 32.0;
      let b_middle = middle as i32;
      for y in b_min_height..=b_max_height {
        let rel = p.chunk_rel().with_y(y);
        let val = {
          const DIV: f64 = 32.0;
          let x = p.x() as f64 / DIV;
          let y = y as f64 / DIV;
          let z = p.z() as f64 / DIV;
          self.noise.get([x, y, z])
        };
        let offset = {
          const DIV: f64 = 256.0;
          let x = p.x() as f64 / DIV;
          let y = y as f64 / DIV;
          let z = p.z() as f64 / DIV;
          (self.offset.get([x, y, z]) + 1.0) / 2.0
        };
        let a = (y as f64 - min_height) / (max_height - min_height);
        let b = if y > b_middle { 0.0 } else { (b_middle - y) as f64 / middle };
        // This is a gradient from top to bottom for how much open space we want.
        let v = a + b;
        // This converts that gradient into a value that we will use to dampen the 3D
        // noise map.
        let mut min = v * 0.2 - 0.1;
        // This dampens the noise more using another noise map, so that we only have
        // caverns in some areas.
        min += offset * 0.1;
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

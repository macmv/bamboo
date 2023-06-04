use crate::{block, world::chunk::MultiChunk};
use bb_common::math::ChunkPos;
use noise::{BasicMulti, NoiseFn, Perlin};

pub struct CaveNoise {
  noise:  BasicMulti<Perlin>,
  middle: BasicMulti<Perlin>,
  offset: BasicMulti<Perlin>,
}

impl CaveNoise {
  pub fn new(_seed: u64) -> Self {
    let mut noise = BasicMulti::<Perlin>::default();
    noise.octaves = 5;
    let mut middle = BasicMulti::<Perlin>::default();
    middle.octaves = 3;
    let mut offset = BasicMulti::<Perlin>::default();
    offset.octaves = 1;
    CaveNoise { noise, middle, offset }
  }

  pub fn carve(&self, pos: ChunkPos, c: &mut MultiChunk) {
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
          const DIV: f64 = 512.0;
          let x = p.x() as f64 / DIV;
          let y = y as f64 / DIV;
          let z = p.z() as f64 / DIV;
          self.offset.get([x, y, z])
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
        min += offset * 0.3 + 0.2;
        if val > min {
          c.set_kind(rel, block::Kind::Air).unwrap();
        }
      }
    }
  }
}

use super::chunk::MultiChunk;
use crate::block;
use common::math::{ChunkPos, Pos};
use noise::{NoiseFn, Perlin};

pub struct Generator {
  noise: Perlin,
}

impl Generator {
  pub fn new() -> Self {
    Self { noise: Perlin::new() }
  }
  pub fn generate(&self, pos: ChunkPos, c: &mut MultiChunk) {
    // This is the height at the middle of the chunk. It is a good average height
    // for the whole chunk.
    let average_height = self.height_at(pos.block() + Pos::new(8, 0, 8)) as i32;
    c.fill_kind(Pos::new(0, 0, 0), Pos::new(15, average_height, 15), block::Kind::Grass).unwrap();
  }
  fn height_at(&self, pos: Pos) -> f64 {
    self.noise.get([pos.x() as f64 / 100.0, pos.z() as f64 / 100.0]) * 30.0 + 60.0
  }
}

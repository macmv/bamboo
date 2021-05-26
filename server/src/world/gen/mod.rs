use super::chunk::MultiChunk;
use crate::block;
use common::math::{ChunkPos, Pos};
use noise::{NoiseFn, Perlin};
use std::cmp::Ordering;

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
    let average_stone_height = self.height_at(pos.block() + Pos::new(8, 0, 8)) as i32 - 5;
    c.fill_kind(Pos::new(0, 0, 0), Pos::new(15, average_stone_height, 15), block::Kind::Stone)
      .unwrap();
    for x in 0..16 {
      for z in 0..16 {
        let height = self.height_at(pos.block() + Pos::new(x, 0, z)) as i32;
        let stone_height = height - 5;
        match stone_height.cmp(&average_stone_height) {
          Ordering::Less => {
            c.fill_kind(
              Pos::new(x, stone_height + 1, z),
              Pos::new(x, average_stone_height, z),
              block::Kind::Air,
            )
            .unwrap();
          }
          Ordering::Greater => {
            c.fill_kind(
              Pos::new(x, average_stone_height, z),
              Pos::new(x, stone_height, z),
              block::Kind::Stone,
            )
            .unwrap();
          }
          _ => {}
        }
        c.fill_kind(Pos::new(x, height - 4, z), Pos::new(x, height - 1, z), block::Kind::Dirt)
          .unwrap();
        c.set_kind(Pos::new(x, height, z), block::Kind::Grass).unwrap();
      }
    }
  }
  fn height_at(&self, pos: Pos) -> f64 {
    self.noise.get([pos.x() as f64 / 100.0, pos.z() as f64 / 100.0]) * 30.0 + 60.0
  }
}

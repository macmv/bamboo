use super::chunk::MultiChunk;
use crate::block;
use common::math::{ChunkPos, Pos};

pub struct Generator {}

impl Generator {
  pub fn new() -> Self {
    Self {}
  }
  pub fn generate(&self, pos: ChunkPos, c: &mut MultiChunk) {
    c.fill_kind(Pos::new(0, 0, 0), Pos::new(15, pos.x() + 30, 15), block::Kind::Grass).unwrap();
  }
}

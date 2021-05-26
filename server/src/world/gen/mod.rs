use super::chunk::MultiChunk;
use crate::block;
use common::math::Pos;

pub struct Generator {}

impl Generator {
  pub fn new() -> Self {
    Self {}
  }
  pub fn generate(&self, c: &mut MultiChunk) {
    c.fill_kind(Pos::new(0, 0, 0), Pos::new(15, 20, 15), block::Kind::Grass).unwrap();
  }
}

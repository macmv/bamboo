use super::chunk::MultiChunk;
use crate::block;
use common::math::Pos;

pub struct Generator {}

impl Generator {
  pub fn new() -> Self {
    Self {}
  }
  pub fn generate(&self, c: &mut MultiChunk) {
    for x in 0..16 {
      for y in 0..20 {
        for z in 0..16 {
          c.set_kind(Pos::new(x, y, z), block::Kind::Grass).unwrap();
        }
      }
    }
  }
}

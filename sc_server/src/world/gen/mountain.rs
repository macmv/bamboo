use super::{BiomeGen, BiomeLayers, WorldGen};
use crate::block;
use sc_common::math::Pos;
use std::cmp;

pub struct Gen {
  id: usize,
}
impl BiomeGen for Gen {
  fn new(id: usize) -> Gen {
    Gen { id }
  }
  fn id(&self) -> usize {
    self.id
  }
  fn layers(&self) -> BiomeLayers {
    let layers = BiomeLayers::new(block::Kind::Bedrock);
    layers
  }
  fn height_at(&self, world: &WorldGen, pos: Pos) -> i32 {
    let dist = world.dist_to_border(pos);
    let mut height = world.height_at(pos) as i32;
    if dist > 12.0 {
      height += (10.0_f64.powi(2) / 10.0) as i32;
      height += ((dist - 12.0).sqrt()) as i32;
    } else if dist > 2.0 {
      height += ((dist - 2.0).powi(2) / 10.0) as i32;
    }
    cmp::min(height, 255)
  }
}

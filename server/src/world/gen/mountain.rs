use super::{BiomeGen, BiomeLayers, WorldGen};
use crate::block;
use common::math::Pos;

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
    world.height_at(pos) as i32 + (dist * 0.5) as i32
  }
}

use super::{BiomeGen, BiomeLayers};
use crate::block;
use common::math::PointGrid;

pub struct Gen {
  id:    usize,
  cacti: PointGrid,
}

impl BiomeGen for Gen {
  fn new(id: usize) -> Gen {
    Gen { id, cacti: PointGrid::new(12345, 16, 10) }
  }
  fn id(&self) -> usize {
    self.id
  }
  fn layers(&self) -> BiomeLayers {
    let mut layers = BiomeLayers::new(block::Kind::Stone);
    layers.add(block::Kind::Sandstone, 5);
    layers.add(block::Kind::Sand, 2);
    layers
  }
}

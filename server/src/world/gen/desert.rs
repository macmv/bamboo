use super::{BiomeGen, BiomeLayers};
use crate::block;
use common::math::PointGrid;

pub struct Gen {
  cacti: PointGrid,
}

impl Gen {
  pub fn new() -> Box<dyn BiomeGen + Send> {
    Box::new(Self { cacti: PointGrid::new(12345, 16, 10) })
  }
}

impl BiomeGen for Gen {
  fn layers(&self) -> BiomeLayers {
    let mut layers = BiomeLayers::new(block::Kind::Stone);
    layers.add(block::Kind::Sandstone, 5);
    layers.add(block::Kind::Sand, 2);
    layers
  }
}

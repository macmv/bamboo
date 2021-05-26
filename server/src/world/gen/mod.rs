use super::chunk::MultiChunk;
use crate::block;
use common::math::{ChunkPos, PointGrid, Pos, Voronoi};
use noise::{NoiseFn, Perlin};
use std::cmp::Ordering;

mod desert;
mod forest;

pub trait BiomeGen {
  fn fill_chunk(&self, world: &WorldGen, pos: ChunkPos, chunk: &mut MultiChunk);
}

pub struct WorldGen {
  biome_map: Voronoi,
  biomes:    Vec<Box<dyn BiomeGen + Send>>,
  height:    Perlin,
}

impl WorldGen {
  pub fn new() -> Self {
    Self {
      biome_map: Voronoi::new(1231451),
      biomes:    vec![desert::Gen::new(), forest::Gen::new()],
      height:    Perlin::new(),
    }
  }
  pub fn generate(&self, pos: ChunkPos, c: &mut MultiChunk) {
    let biome = self.biome_map.get(pos.block().x(), pos.block().z()) as usize % self.biomes.len();
    self.biomes[biome].fill_chunk(self, pos, c);
  }
  pub fn height_at(&self, pos: Pos) -> f64 {
    self.height.get([pos.x() as f64 / 100.0, pos.z() as f64 / 100.0]) * 30.0 + 60.0
  }
}

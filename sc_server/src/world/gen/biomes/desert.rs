use super::{
  super::{math::PointGrid, BiomeGen, BiomeLayers},
  WorldGen,
};
use crate::{block, world::chunk::MultiChunk};
use sc_common::math::{ChunkPos, Pos};
use std::collections::HashMap;

pub struct Gen {
  id:    usize,
  cacti: PointGrid,
}

impl Gen {
  pub fn place_cactus(&self, world: &WorldGen, c: &mut MultiChunk, pos: Pos) {
    let height;
    if world.chance(pos, 0.50) {
      height = 3
    } else {
      height = 2
    }
    let rel = pos.chunk_rel();
    c.fill_kind(rel, rel.add_y(height), block::Kind::Cactus).unwrap();
  }
}

impl BiomeGen for Gen {
  fn new(id: usize) -> Gen { Gen { id, cacti: PointGrid::new(12345, 16, 10) } }
  fn id(&self) -> usize { self.id }
  fn layers(&self) -> BiomeLayers {
    let mut layers = BiomeLayers::new(block::Kind::Stone);
    layers.add(block::Kind::Sandstone, 5);
    layers.add(block::Kind::Sand, 2);
    layers
  }
  fn decorate(
    &self,
    world: &WorldGen,
    chunk_pos: ChunkPos,
    c: &mut MultiChunk,
    tops: &HashMap<Pos, i32>,
  ) {
    for mut p in chunk_pos.columns() {
      if world.is_biome(self, p) {
        let height = tops[&p.chunk_rel()];
        p = p.with_y(height + 1);
        if self.cacti.contains(p.into()) {
          self.place_cactus(world, c, p);
        } else if world.chance(p, 0.01) {
          c.set_kind(p.chunk_rel(), block::Kind::DeadBush).unwrap();
        }
      }
    }
  }
}

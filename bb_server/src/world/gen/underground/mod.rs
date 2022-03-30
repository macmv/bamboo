use super::WorldGen;
use crate::world::chunk::MultiChunk;
use bb_common::math::ChunkPos;

mod caves;
mod ores;

use caves::CaveGen;
use ores::OreGen;

#[allow(unused)]
pub struct Underground {
  ores:  OreGen,
  caves: CaveGen,
}

impl Underground {
  pub fn new(seed: u64) -> Self {
    Underground { ores: OreGen::new(seed), caves: CaveGen::new(seed) }
  }

  #[allow(unused)]
  pub fn process(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    self.ores.place(world, pos, c);
    self.caves.carve(world, pos, c);
  }
}

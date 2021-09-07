use super::WorldGen;
use crate::world::chunk::MultiChunk;
use common::math::ChunkPos;

mod caves;
mod ores;

use caves::CaveGen;
use ores::OreGen;

pub struct Underground {
  caves: CaveGen,
  ores:  OreGen,
}

impl Underground {
  pub fn new(seed: u64) -> Self {
    Underground { caves: CaveGen::new(seed), ores: OreGen::new(seed) }
  }

  pub fn process(&mut self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    self.ores.place(world, pos, c);
    self.caves.carve(world, pos, c);
  }
}

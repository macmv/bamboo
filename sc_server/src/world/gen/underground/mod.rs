use super::WorldGen;
use crate::world::chunk::MultiChunk;
use sc_common::math::ChunkPos;

mod caves;
mod ores;

use caves::CaveGen;
use ores::OreGen;

pub struct Underground {
  ores:  OreGen,
  caves: CaveGen,
}

impl Underground {
  pub fn new(seed: u64) -> Self {
    Underground { ores: OreGen::new(seed), caves: CaveGen::new(seed) }
  }

  pub fn process(&mut self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    self.ores.place(world, pos, c);
    self.caves.carve(world, pos, c);
  }
}

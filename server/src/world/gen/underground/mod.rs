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

  pub fn process(&self, pos: ChunkPos, c: &mut MultiChunk) {
    self.ores.place(pos, c);
    self.caves.carve(pos, c);
  }
}

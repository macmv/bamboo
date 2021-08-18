use crate::world::chunk::MultiChunk;
use common::math::{ChunkPos, WyhashRng};

pub struct OreGen {}

impl OreGen {
  pub fn new(seed: u64) -> Self {
    OreGen {}
  }

  pub fn place(&self, pos: ChunkPos, c: &mut MultiChunk) {
    for col in pos.columns() {}
  }
}

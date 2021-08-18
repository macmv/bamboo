use super::super::WorldGen;
use crate::world::chunk::MultiChunk;
use common::math::ChunkPos;

pub struct CaveGen {}

impl CaveGen {
  pub fn new(seed: u64) -> Self {
    CaveGen {}
  }

  pub fn carve(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {}
}

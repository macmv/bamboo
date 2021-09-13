use super::super::WorldGen;
use crate::{block, world::chunk::MultiChunk};
use sc_common::math::ChunkPos;

pub struct OreGen {}

impl OreGen {
  pub fn new(seed: u64) -> Self {
    OreGen {}
  }

  pub fn place(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for col in pos.columns() {
      for y in 0..(world.height_at(col) as i32) {
        let p = col.with_y(y);
        if world.chance(p, 0.05) {
          c.set_kind(p.chunk_rel(), block::Kind::CoalOre).unwrap();
        }
      }
    }
  }
}

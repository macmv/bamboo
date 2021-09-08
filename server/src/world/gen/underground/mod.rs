use super::WorldGen;
use crate::world::chunk::MultiChunk;
use common::math::ChunkPos;
use std::cell::RefCell;

mod caves;
mod ores;

use caves::CaveGen;
use ores::OreGen;

pub struct Underground {
  ores: OreGen,
}

thread_local!(static CAVES: RefCell<CaveGen> = RefCell::new(CaveGen::new(1235623456)));

impl Underground {
  pub fn new(seed: u64) -> Self {
    Underground { ores: OreGen::new(seed) }
  }

  pub fn process(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    self.ores.place(world, pos, c);
    CAVES.with(|caves| caves.borrow_mut().carve(world, pos, c));
  }
}

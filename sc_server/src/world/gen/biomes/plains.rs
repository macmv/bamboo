use super::{super::BiomeGen, WorldGen};
use crate::{block, world::chunk::MultiChunk};
use sc_common::math::{ChunkPos, Pos};
use std::collections::HashMap;

pub struct Gen {
  id: usize,
}
impl BiomeGen for Gen {
  fn new(id: usize) -> Gen { Gen { id } }
  fn id(&self) -> usize { self.id }
  fn decorate(
    &self,
    world: &WorldGen,
    _chunk_pos: ChunkPos,
    c: &mut MultiChunk,
    tops: &HashMap<Pos, usize>,
  ) {
    for (&p, &biome) in tops {
      if biome == self.id() {
        let p = p + Pos::new(0, 1, 0);
        if world.chance(p, 0.30)
          && matches!(
            c.get_kind(p.chunk_rel().with_y(p.y - 1)).unwrap(),
            block::Kind::GrassBlock | block::Kind::Dirt
          )
        {
          c.set_kind(p.chunk_rel(), block::Kind::Grass).unwrap();
        }
      }
    }
  }
}

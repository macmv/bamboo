use super::{BiomeGen, BiomeLayers, WorldGen};
use crate::{block, block::Kind, world::chunk::MultiChunk};
use sc_common::math::Pos;

pub struct Gen {
  id: usize,
}

impl BiomeGen for Gen {
  fn new(id: usize) -> Gen { Gen { id } }
  fn id(&self) -> usize { self.id }
  fn layers(&self) -> BiomeLayers {
    let mut layers = BiomeLayers::new(block::Kind::Stone);
    layers.add(block::Kind::Gravel, 2);
    layers.add(block::Kind::Dirt, 2);
    layers
  }
  fn fill_column(&self, world: &WorldGen, pos: Pos, c: &mut MultiChunk) {
    let height = self.height_at(world, pos) as i32;
    let rh = world.river_height_at(pos) as i32;
    let pos = pos.chunk_rel();
    let layers = self.layers();
    let min_height = height - layers.total_height() as i32;
    c.fill_kind(pos, pos + Pos::new(0, min_height, 0), layers.main_area).unwrap();
    let mut level = min_height as u32;
    for (k, depth) in layers.layers() {
      c.fill_kind(
        pos + Pos::new(0, level as i32, 0),
        pos + Pos::new(0, (level + depth) as i32, 0),
        *k,
      )
      .unwrap();
      level += depth;
    }
    if height < rh {
      c.fill_kind(pos + Pos::new(0, height, 0), pos + Pos::new(0, rh, 0), Kind::Water).unwrap();
    }
  }
}

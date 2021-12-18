use super::super::{
  math::{Point, PointGrid},
  util::Cache,
  WorldGen,
};
use crate::{block, util::Threaded, world::chunk::MultiChunk};
use sc_common::math::{ChunkPos, Pos};

#[derive(Debug)]
pub struct OreGen {
  ores: Vec<Ore>,
}

#[derive(Debug)]
struct Ore {
  origins: PointGrid,
  veins:   Threaded<Cache<Point, Vein>>,
  kind:    block::Kind,
}

#[derive(Debug)]
struct Vein {
  pos: Pos,
}

impl OreGen {
  pub fn new(seed: u64) -> Self { OreGen { ores: vec![Ore::new(seed, block::Kind::CoalOre)] } }

  pub fn place(&self, _world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for o in &self.ores {
      o.place(pos, c);
    }
  }
}

impl Ore {
  pub fn new(seed: u64, kind: block::Kind) -> Self {
    Ore {
      origins: PointGrid::new(seed, 256, 8),
      veins: Threaded::new(move || {
        Cache::new(move |origin: Point| Vein { pos: Pos::new(origin.x, 80, origin.y) })
      }),
      kind,
    }
  }

  pub fn place(&self, pos: ChunkPos, c: &mut MultiChunk) {
    for origin in self.origins.neighbors(Point::new(pos.block_x(), pos.block_z()), 1) {
      self.veins.get(|cache| {
        let vein = cache.get(origin);
        vein.place(pos, c, self.kind);
      });
      // self.carve_cave_tree(origin, pos, c);
    }
  }
}

impl Vein {
  pub fn place(&self, pos: ChunkPos, c: &mut MultiChunk, kind: block::Kind) {
    if self.pos.chunk() == pos {
      c.set_kind(self.pos.chunk_rel(), kind).unwrap();
    }
  }
}

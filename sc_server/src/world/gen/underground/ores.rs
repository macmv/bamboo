use super::super::{util::Cache, WorldGen, WyhashRng};
use crate::{
  block,
  math::{Point, PointGrid},
  util::Threaded,
  world::chunk::MultiChunk,
};
use sc_common::math::{ChunkPos, Pos, RngCore};

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
  blocks: Vec<Pos>,
}

impl OreGen {
  pub fn new(seed: u64) -> Self {
    OreGen {
      ores: vec![
        Ore::new(seed, block::Kind::CoalOre, 2, 16),
        Ore::new(seed, block::Kind::IronOre, 4, 16),
        Ore::new(seed, block::Kind::GoldOre, 8, 4),
      ],
    }
  }

  pub fn place(&self, _world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    for o in &self.ores {
      o.place(pos, c);
    }
  }
}

impl Ore {
  /// Creates a new ore generator.
  ///
  /// The `space` is the number of blocks between every ore vein on each axis.
  /// For example, a value of `2` would be used to create an ore vein every 2
  /// blocks on the X-Z plane. The height of each vein is randomized, so this
  /// would be used for something like coal.
  ///
  /// The `size` is the maximum amount of ores to place in one vein. This is
  /// randomized, but will always be at most `size` blocks.
  pub fn new(seed: u64, kind: block::Kind, space: u32, size: u32) -> Self {
    Ore {
      origins: PointGrid::new(seed, 256, space),
      veins: Threaded::new(move || Cache::new(move |origin: Point| Vein::new(seed, origin, size))),
      kind,
    }
  }

  pub fn place(&self, pos: ChunkPos, c: &mut MultiChunk) {
    // We assume each ore vein is 5x5 blocks at most. This means, in order to get
    // all the ore veins in this chunk, we go 10 blocks out from the center of the
    // chunk.
    for origin in self.origins.neighbors(Point::new(pos.block_x(), pos.block_z()), 10) {
      self.veins.get(|cache| {
        let vein = cache.get(origin);
        vein.place(pos, c, self.kind);
      });
      // self.carve_cave_tree(origin, pos, c);
    }
  }
}

impl Vein {
  pub fn new(seed: u64, origin: Point, size: u32) -> Self {
    let mut rng = WyhashRng::new(seed ^ ((origin.x as u64) << 32) ^ origin.y as u64);
    let mut pos = Pos::new(origin.x, (rng.next_u32() % 64) as i32, origin.y);
    let mut blocks = vec![];
    for i in 0..size {
      let offset = match rng.next_u32() % 6 {
        0 => Pos::new(1, 0, 0),
        1 => Pos::new(-1, 0, 0),
        2 => Pos::new(0, 1, 0),
        3 => Pos::new(0, -1, 0),
        4 => Pos::new(0, 0, 1),
        5 => Pos::new(0, 0, -1),
        _ => unreachable!(),
      };
      pos += offset;
      if pos.y < 0 {
        pos.y = 0;
      }
      blocks.push(pos);
      if rng.next_u32() % size * 2 < i {
        break;
      }
    }
    Vein { blocks }
  }
  pub fn place(&self, pos: ChunkPos, c: &mut MultiChunk, kind: block::Kind) {
    for p in &self.blocks {
      let rel = p.chunk_rel();
      if p.chunk() == pos && c.get_kind(rel).unwrap() == block::Kind::Stone {
        c.set_kind(rel, kind).unwrap();
      }
    }
  }
}

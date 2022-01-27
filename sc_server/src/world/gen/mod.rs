use super::chunk::MultiChunk;
use crate::block;
use math::WarpedVoronoi;
use noise::{BasicMulti, NoiseFn};
use sc_common::{
  config::Config,
  math::{ChunkPos, Pos, RngCore, WyhashRng},
  version::BlockVersion,
};
use std::{
  cmp::Ordering,
  collections::{HashMap, HashSet},
};

mod biomes;
mod math;
mod sl;
mod underground;
pub mod util;

pub use sl::SlBiomeGen;

use underground::Underground;

pub struct BiomeLayers {
  layers:       Vec<(block::Kind, u32)>,
  main_area:    block::Kind,
  total_height: u32, // Sum of layers
}

impl BiomeLayers {
  /// Creates a new biome layer definition. The main area should probably be
  /// stone; it is what will be used to fill all the blocks under the actual
  /// layers.
  pub fn new(main_area: block::Kind) -> Self {
    BiomeLayers { main_area, layers: vec![], total_height: 0 }
  }

  /// Adds a new layer. This will be placed ontop of the previous layer.
  ///
  /// # Example
  ///
  /// This would create the default block layers you would see in a forest or
  /// plains.
  ///
  /// ```rust
  /// let mut layers = BiomeLayers::new(block::Kind::Stone);
  /// layers.add(block::Kind::Dirt, 4);
  /// layers.add(block::Kind::GrassBlock, 1);
  /// ```
  pub fn add(&mut self, kind: block::Kind, height: u32) {
    self.layers.push((kind, height));
    self.total_height += height;
  }

  /// Returns the total height of all defined layers.
  pub fn total_height(&self) -> u32 { self.total_height }

  /// Returns the internal layers list
  pub fn layers(&self) -> &[(block::Kind, u32)] { &self.layers }

  /// Returns the block kind at the given height. This should never fail.
  pub fn get(&self, mut depth: u32) -> block::Kind {
    if depth > self.total_height {
      self.main_area
    } else {
      for (kind, d) in self.layers.iter().rev() {
        match depth.checked_sub(*d) {
          Some(new_depth) => depth = new_depth,
          None => return *kind,
        }
      }
      self.main_area
    }
  }
}

pub trait BiomeGen {
  /// Creates a new biome generator, with the given id. This id must be returned
  /// by [`id`](Self::id).
  fn new(id: usize) -> Self
  where
    Self: Sized;
  /// Returns this biome's id. This is used to check if a type is the correct
  /// biome, so returning the wrong thing here will break things.
  fn id(&self) -> usize;
  /// This fills an entire chunk with the given biome. This will fill the chunk
  /// with stone, up to the height at the middle. It will then carve/add blocks
  /// to the other columns of the chunk. Finally, it will call [`fill_column`]
  /// for each column within the chunk. It will use the height of the stone as
  /// the minimum to pass to `fill_column`.
  ///
  /// For most biomes, this should not be overriden. [`height_at`] should be
  /// overriden if you want to build something like a mountain, and [`layers`]
  /// should be overriden if you need something like a desert.
  fn fill_chunk(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    let average_min_height = self.height_at(world, pos.block() + Pos::new(8, 0, 8));
    let layers = self.layers();
    c.fill_kind(Pos::new(0, 0, 0), Pos::new(15, average_min_height, 15), layers.main_area).unwrap();
    for x in 0..16 {
      for z in 0..16 {
        let height = self.height_at(world, pos.block() + Pos::new(x, 0, z)) as i32;
        let min_height = height - layers.total_height() as i32;
        match min_height.cmp(&average_min_height) {
          Ordering::Less => {
            c.fill_kind(
              Pos::new(x, min_height + 1, z),
              Pos::new(x, average_min_height, z),
              block::Kind::Air,
            )
            .unwrap();
          }
          Ordering::Greater => {
            c.fill_kind(
              Pos::new(x, average_min_height, z),
              Pos::new(x, min_height, z),
              layers.main_area,
            )
            .unwrap();
          }
          _ => {}
        }
        let mut level = min_height as u32;
        for (k, depth) in &layers.layers {
          c.fill_kind(Pos::new(x, level as i32, z), Pos::new(x, (level + depth) as i32, z), *k)
            .unwrap();
          level += depth;
        }
      }
    }
  }
  /// This fills a single block column with this chunk. If there are multiple
  /// biomes in a chunk, this is called instead of fill_chunk. Y of pos will
  /// always be 0.
  fn fill_column(&self, world: &WorldGen, pos: Pos, c: &mut MultiChunk) {
    let height = self.height_at(world, pos);
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
  }
  /// Decorates the given chunk. This is called for every chunk where a block of
  /// this biome is in the radius of [`decorate_radius`].
  ///
  /// The tops map is a mapping of every block column in the chunk to maximum Y
  /// values.
  fn decorate(
    &self,
    _world: &WorldGen,
    _pos: ChunkPos,
    _c: &mut MultiChunk,
    _tops: &HashMap<Pos, usize>,
  ) {
  }
  /// Returns the longest distance that decorations can extend outside of this
  /// biome. This is used to call [`decorate`] when there are blocks of this
  /// biome in a nearby chunk.
  fn decorate_radius(&self) -> u32 { 0 }
  /// Returns the layers that should be used to fill in the ground. See
  /// [`BiomeLayer`] for more.
  fn layers(&self) -> BiomeLayers {
    let mut layers = BiomeLayers::new(block::Kind::Stone);
    layers.add(block::Kind::Dirt, 4);
    layers.add(block::Kind::GrassBlock, 1);
    layers
  }
  /// Returns this biome's height at the given position. By default, this just
  /// uses the world height at the given position.
  fn height_at(&self, world: &WorldGen, pos: Pos) -> i32 { 64 }
}

impl Default for WorldGen {
  fn default() -> Self { Self::new() }
}

pub struct WorldGen {
  seed:        u64,
  biome_map:   WarpedVoronoi,
  biomes:      Vec<Box<dyn BiomeGen + Send + Sync>>,
  stone:       BasicMulti,
  max_height:  BasicMulti,
  underground: Underground,
  debug:       bool,
}

impl WorldGen {
  pub fn new() -> Self {
    let mut stone = BasicMulti::new();
    stone.octaves = 3;
    let mut max_height = BasicMulti::new();
    max_height.octaves = 1;
    let seed = 3210471203948712039;
    WorldGen {
      seed,
      biome_map: WarpedVoronoi::new(seed),
      biomes: vec![],
      stone,
      max_height,
      underground: Underground::new(seed),
      debug: false,
    }
  }
  pub fn from_config(config: &Config) -> Self {
    if config.get("world.debug") {
      let mut gen = WorldGen::new();
      gen.debug = true;
      gen
    } else if config.get("world.void") {
      WorldGen::new()
    } else {
      let mut gen = WorldGen::new();
      for biome in config.get::<str, Vec<&str>>("world.biomes") {
        if gen.add_named_biome(biome).is_err() {
          warn!("unknown biome '{}', skipping", biome);
        }
      }
      gen
    }
  }
  pub fn add_biome<B: BiomeGen + Send + Sync + 'static>(&mut self) {
    let id = self.biomes.len();
    self.biomes.push(Box::new(B::new(id)));
  }

  pub fn generate(&self, pos: ChunkPos, c: &mut MultiChunk) {
    if self.debug {
      let ver = match pos.z() {
        0 => BlockVersion::V1_8,
        1 => BlockVersion::V1_9,
        2 => BlockVersion::V1_10,
        3 => BlockVersion::V1_11,
        4 => BlockVersion::V1_12,
        6 => BlockVersion::V1_14,
        7 => BlockVersion::V1_15,
        8 => BlockVersion::V1_16,
        9 => BlockVersion::V1_17,
        10 => BlockVersion::V1_18,
        _ => return,
      };
      let x = pos.block_x();
      if x >= 0 && x < 1000 {
        for x in pos.block_x()..pos.block_x() + 16 {
          for state in 0..16 {
            let ty = c.type_converter().type_from_id(x as u32 * 16 + state as u32, ver);
            c.set_type(Pos::new(x % 16, 0, state), ty).unwrap();
          }
        }
      }
      return;
    }
    // Fast path for void worlds
    if self.biomes.is_empty() {
      return;
    }
    c.enable_lighting(false);
    let div = 32.0;
    let min_height = 40.0_f64;
    let b_min_height = min_height.floor() as i32;
    c.fill_kind(Pos::new(0, 0, 0), Pos::new(15, b_min_height, 15), block::Kind::Stone).unwrap();
    let mut biomes = HashSet::new();
    let mut tops = HashMap::new();
    for p in pos.columns() {
      let x = p.x() as f64 / 64.0;
      let z = p.z() as f64 / 64.0;
      let max_height = (self.max_height.get([x, z]) * 1.0) / 2.0 * 50.0 + 200.0;
      let b_max_height = max_height.ceil() as i32;
      let biome = self.biome_id_at(p);
      let layers = self.biomes[biome].layers();
      biomes.insert(biome);
      let mut depth = 0;
      for y in (b_min_height..=b_max_height).rev() {
        let rel = p.chunk_rel().with_y(y);
        let val = {
          let x = p.x() as f64 / div / 2.0;
          let y = y as f64 / div;
          let z = p.z() as f64 / div / 2.0;
          self.stone.get([x, y, z])
        };
        let mut min = (y as f64 - min_height) / (max_height - min_height);
        min = min * 2.0 - 1.0;
        if val > min {
          if depth == 0 {
            tops.insert(p.with_y(y), biome);
          }
          c.set_kind(rel, layers.get(depth)).unwrap();
          depth += 1;
        } else {
          depth = 0;
        }
      }
    }
    for b in &biomes {
      self.biomes[*b].decorate(self, pos, c, &tops);
    }
    c.enable_lighting(true);
    /*
    if biomes.len() == 1 {
      for b in &biomes {
        self.biomes[*b].fill_chunk(self, pos, c);
      }
    } else {
      for p in pos.columns() {
        let biome = self.biome_map.get(p.into()) as usize % self.biomes.len();
        self.biomes[biome].fill_column(self, p, c);
      }
    }
    */
  }
  pub fn biome_id_at(&self, pos: Pos) -> usize {
    self.biome_map.get(pos.into()) as usize % self.biomes.len()
  }
  pub fn dist_to_border(&self, pos: Pos) -> f64 { self.biome_map.dist_to_border(pos.into()) }
  pub fn is_biome<B: BiomeGen>(&self, b: &B, pos: Pos) -> bool {
    let actual = self.biome_id_at(pos);
    b.id() == actual
  }

  /// Seeds a RNG with the position, then returns a true `percent` amount of the
  /// time. This should be used to place grass, randomize trees, etc. It is
  /// position dependant, so that chunks can generate in any order, and they
  /// will still be the same.
  pub fn chance(&self, pos: Pos, percent: f32) -> bool {
    let mut rng = WyhashRng::new(
      0xe6cc56f1f7550d95_u64
        .wrapping_mul(self.seed)
        .wrapping_mul(pos.x() as u64)
        .wrapping_mul((pos.z() as u64) << 32)
        .wrapping_mul(pos.y() as u64),
    );
    rng.next_u64() % 1000 < (percent * 1000.0) as u64
  }
}

use super::chunk::MultiChunk;
use crate::block;
use common::math::{ChunkPos, Pos, Voronoi};
use noise::{NoiseFn, Perlin};
use std::{cmp::Ordering, collections::HashSet};

mod desert;
mod forest;

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

  /// Returns the layer at a given index.
  pub fn get(&self, i: usize) -> Option<&(block::Kind, u32)> {
    self.layers.get(i)
  }

  /// Returns the total height of all defined layers.
  pub fn total_height(&self) -> u32 {
    self.total_height
  }
}

pub trait BiomeGen {
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
        let height = world.height_at(pos.block() + Pos::new(x, 0, z)) as i32;
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
    for (k, depth) in &layers.layers {
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
  fn decorate(&self, _world: &WorldGen, _pos: ChunkPos, _c: &mut MultiChunk) {}
  /// Returns the longest distance that decorations can extend outside of this
  /// biome. This is used to call [`decorate`] when there are blocks of this
  /// biome in a nearby chunk.
  fn decorate_radius(&self) -> u32 {
    0
  }
  /// Returns the layers that should be used to fill in the ground. See
  /// [`BiomeLayer`] for more.
  fn layers(&self) -> BiomeLayers {
    let mut layers = BiomeLayers::new(block::Kind::Stone);
    layers.add(block::Kind::Dirt, 4);
    layers.add(block::Kind::Grass, 4);
    layers
  }
  /// Returns this biome's height at the given position. By default, this just
  /// uses the world height at the given position.
  fn height_at(&self, world: &WorldGen, pos: Pos) -> i32 {
    world.height_at(pos) as i32
  }
}

pub struct WorldGen {
  biome_map: Voronoi,
  biomes:    Vec<Box<dyn BiomeGen + Send>>,
  height:    Perlin,
}

impl WorldGen {
  pub fn new() -> Self {
    Self {
      biome_map: Voronoi::new(1231451),
      biomes:    vec![desert::Gen::new(), forest::Gen::new()],
      height:    Perlin::new(),
    }
  }
  pub fn generate(&self, pos: ChunkPos, c: &mut MultiChunk) {
    let mut biomes = HashSet::new();
    for x in 0..16 {
      for z in 0..16 {
        let biome =
          self.biome_map.get(pos.block().x() + x, pos.block().z() + z) as usize % self.biomes.len();
        biomes.insert(biome);
      }
    }
    if biomes.len() == 1 {
      for b in &biomes {
        self.biomes[*b].fill_chunk(self, pos, c);
      }
    } else {
      for x in 0..16 {
        for z in 0..16 {
          let biome = self.biome_map.get(pos.block().x() + x, pos.block().z() + z) as usize
            % self.biomes.len();
          self.biomes[biome].fill_column(self, pos.block() + Pos::new(x, 0, z), c);
        }
      }
    }
    for b in &biomes {
      self.biomes[*b].decorate(self, pos, c);
    }
  }
  pub fn height_at(&self, pos: Pos) -> f64 {
    self.height.get([pos.x() as f64 / 100.0, pos.z() as f64 / 100.0]) * 30.0 + 60.0
  }
}

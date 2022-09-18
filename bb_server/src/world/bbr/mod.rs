//! Bamboo region storing in memory, reading, and writing to disk.

mod fs;

use super::{CountedChunk, World};
use bb_common::math::ChunkPos;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{
  collections::HashMap,
  sync::{Arc, Weak},
};

/// The same structure as a chunk position, but used to index into a region. Can
/// be converted to/from a `ChunkPos` by multiplying/dividing its coordinates by
/// 32.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionPos {
  pub x: i32,
  pub z: i32,
}

/// A chunk position within a region. The X and Z cannot be outside `0..32`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionRelPos {
  x: u8,
  z: u8,
}

pub struct RegionMap {
  world:   Weak<World>,
  regions: RwLock<HashMap<RegionPos, Region>>,
  save:    bool,
}

pub struct Region {
  world:  Arc<World>,
  pos:    RegionPos,
  /// An array of `32*32 = 1024` chunks. The index is `x + z * 32`.
  chunks: RwLock<[Option<CountedChunk>; 1024]>,
  save:   bool,
}

impl RegionMap {
  pub fn new(world: Weak<World>, save: bool) -> Self {
    RegionMap { world, regions: RwLock::new(HashMap::new()), save }
  }

  pub fn region<F: FnOnce(&Region) -> R, R>(&self, pos: ChunkPos, f: F) -> R {
    let lock = self.regions.read();
    let region_pos = RegionPos::new(pos);
    let rlock = if !lock.contains_key(&region_pos) {
      drop(lock);
      let mut write = self.regions.write();
      // If someone else got the write lock, and wrote this region, we don't
      // want to write it twice.
      write
        .entry(region_pos)
        .or_insert_with(|| Region::new_load(self.world.upgrade().unwrap(), region_pos, self.save));
      RwLockWriteGuard::downgrade(write)
    } else {
      lock
    };
    let region = rlock.get(&region_pos).unwrap();
    f(region)
  }

  pub fn has_chunk(&self, pos: ChunkPos) -> bool {
    let lock = self.regions.read();
    if let Some(region) = lock.get(&RegionPos::new(pos)) {
      region.has_chunk(RegionRelPos::new(pos))
    } else {
      false
    }
  }

  pub fn unload_chunks(&self) {
    let mut unloadable = vec![];
    {
      let rl = self.regions.read();
      for (pos, region) in rl.iter() {
        if region.unload_chunks() {
          unloadable.push(*pos);
        }
      }
    }
    if !unloadable.is_empty() {
      let mut wl = self.regions.write();
      for pos in unloadable {
        wl.remove(&pos);
      }
    }
  }

  pub fn save(&self) {
    if !self.save {
      info!("saving disabled, skipping");
      return;
    }
    info!("saving world...");
    let lock = self.regions.read();
    for region in lock.values() {
      region.save();
    }
    info!("saved");
  }
}

impl Region {
  fn new(world: Arc<World>, pos: RegionPos, save: bool) -> Self {
    const NONE: Option<CountedChunk> = None;
    Region { world, pos, chunks: RwLock::new([NONE; 1024]), save }
  }
  pub fn new_load(world: Arc<World>, pos: RegionPos, save: bool) -> Self {
    let mut region = Region::new(world, pos, save);
    region.load();
    region
  }

  pub fn get<R>(&self, pos: RegionRelPos, f: impl FnOnce(&Option<CountedChunk>) -> R) -> R {
    f(&self.chunks.read()[pos.x as usize + pos.z as usize * 32])
  }
  fn get_mut<R>(&self, pos: RegionRelPos, f: impl FnOnce(&mut Option<CountedChunk>) -> R) -> R {
    f(&mut self.chunks.write()[pos.x as usize + pos.z as usize * 32])
  }
  pub fn get_or_generate<R>(
    &self,
    pos: impl Into<RegionRelPos>,
    gen: impl FnOnce() -> CountedChunk,
    f: impl FnOnce(&CountedChunk) -> R,
  ) -> R {
    let pos = pos.into();
    let rlock = self.chunks.read();
    if let Some(c) = &rlock[pos.x as usize + pos.z as usize * 32] {
      f(c)
    } else {
      drop(rlock);
      self.get_mut(pos, |c| {
        *c = Some(gen());
        f(c.as_mut().unwrap())
      })
    }
  }
  pub fn has_chunk(&self, pos: impl Into<RegionRelPos>) -> bool {
    self.get(pos.into(), |c| c.is_some())
  }
  /// Returns true if this region can be unloaded.
  pub fn unload_chunks(&self) -> bool {
    // If all the chunks are either `None` or viewed by nobody, we can unload this
    // region.
    for c in self.chunks.read().iter().flatten() {
      if c.count.load(std::sync::atomic::Ordering::Acquire) != 0 {
        return false;
      }
    }
    true
  }
}

impl Drop for Region {
  /// Saves the region on drop. This makes unloading regions easy.
  fn drop(&mut self) { self.save(); }
}

impl RegionPos {
  pub fn new(chunk: ChunkPos) -> Self {
    RegionPos {
      x: if chunk.x() < 0 { (chunk.x() + 1) / 32 - 1 } else { chunk.x() / 32 },
      z: if chunk.z() < 0 { (chunk.z() + 1) / 32 - 1 } else { chunk.z() / 32 },
    }
  }
}

impl RegionRelPos {
  pub fn new(chunk: ChunkPos) -> Self {
    RegionRelPos { x: ((chunk.x() % 32 + 32) % 32) as u8, z: ((chunk.z() % 32 + 32) % 32) as u8 }
  }
}

impl From<ChunkPos> for RegionPos {
  fn from(chunk: ChunkPos) -> Self { RegionPos::new(chunk) }
}
impl From<ChunkPos> for RegionRelPos {
  fn from(chunk: ChunkPos) -> Self { RegionRelPos::new(chunk) }
}

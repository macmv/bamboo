//! Bamboo region storing in memory, reading, and writing to disk.

mod fs;

use super::CountedChunk;
use bb_common::math::ChunkPos;
use parking_lot::{Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use std::collections::HashMap;

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
  regions: RwLock<HashMap<RegionPos, Mutex<Region>>>,
  save:    bool,
}

pub struct Region {
  pos:    RegionPos,
  /// An array of `32*32 = 1024` chunks. The index is `x + z * 32`.
  chunks: Box<[Option<CountedChunk>; 1024]>,
  save:   bool,
}

impl RegionMap {
  pub fn new(save: bool) -> Self { RegionMap { regions: RwLock::new(HashMap::new()), save } }

  pub fn region<F: FnOnce(MutexGuard<Region>) -> R, R>(
    &self,
    pos: ChunkPos,
    new_chunk: impl Fn() -> CountedChunk,
    f: F,
  ) -> R {
    let lock = self.regions.read();
    let region_pos = RegionPos::new(pos);
    let rlock = if !lock.contains_key(&region_pos) {
      drop(lock);
      let mut write = self.regions.write();
      // If someone else got the write lock, and wrote this region, we don't
      // want to write it twice.
      write
        .entry(region_pos)
        .or_insert_with(|| Mutex::new(Region::new_load(new_chunk, region_pos, self.save)));
      RwLockWriteGuard::downgrade(write)
    } else {
      lock
    };
    let region = rlock.get(&region_pos).unwrap();
    f(region.lock())
  }

  pub fn has_chunk(&self, pos: ChunkPos) -> bool {
    let lock = self.regions.read();
    if let Some(region) = lock.get(&RegionPos::new(pos)) {
      region.lock().has_chunk(RegionRelPos::new(pos))
    } else {
      false
    }
  }

  pub fn unload_chunks(&self) {
    let mut unloadable = vec![];
    {
      let rl = self.regions.read();
      for (pos, region) in rl.iter() {
        if region.lock().unload_chunks() {
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
      region.lock().save();
    }
    info!("saved");
  }
}

impl Region {
  fn new(pos: RegionPos, save: bool) -> Self {
    const NONE: Option<CountedChunk> = None;
    Region { pos, chunks: Box::new([NONE; 1024]), save }
  }
  pub fn new_load(new_chunk: impl Fn() -> CountedChunk, pos: RegionPos, save: bool) -> Self {
    let mut region = Region::new(pos, save);
    region.load(new_chunk);
    region
  }

  pub fn get(&self, pos: RegionRelPos) -> &Option<CountedChunk> {
    &self.chunks[pos.x as usize + pos.z as usize * 32]
  }
  fn get_mut(&mut self, pos: RegionRelPos) -> &mut Option<CountedChunk> {
    &mut self.chunks[pos.x as usize + pos.z as usize * 32]
  }
  pub fn get_or_generate(
    &mut self,
    pos: impl Into<RegionRelPos>,
    gen: impl FnOnce() -> CountedChunk,
  ) -> &mut CountedChunk {
    match self.get_mut(pos.into()) {
      Some(c) => c,
      chunk @ None => {
        *chunk = Some(gen());
        chunk.as_mut().unwrap()
      }
    }
  }
  pub fn has_chunk(&self, pos: impl Into<RegionRelPos>) -> bool { self.get(pos.into()).is_some() }
  /// Returns true if this region can be unloaded.
  pub fn unload_chunks(&mut self) -> bool {
    // If all the chunks are either `None` or viewed by nobody, we can unload this
    // region.
    for c in self.chunks.iter().flatten() {
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

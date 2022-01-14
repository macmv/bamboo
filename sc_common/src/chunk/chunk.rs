use super::section::Section;

use crate::math::{Pos, PosError};
use std::cmp;

/// A chunk column. This is not clone, because that would mean duplicating an
/// entire chunk, which you probably don't want to do. If you do need to clone a
/// chunk, use [`Chunk::duplicate()`].
///
/// If you want to create a cross-versioned chunk, use [`MultiChunk`] instead.
pub struct Chunk<S: Section> {
  sections: Vec<Option<S>>,
}

impl<S: Section> Chunk<S> {
  pub fn new() -> Self { Chunk { sections: Vec::new() } }
  pub fn from_bitmap(bitmap: u16, mut sections: Vec<S>) -> Self {
    let mut arr: Vec<Option<S>> = (0..16).map(|_| None).collect();
    for i in (0..16).rev() {
      if bitmap & (1 << i) != 0 {
        arr[i] = Some(sections.pop().unwrap());
      }
    }
    Chunk { sections: arr }
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  pub fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    let index = pos.chunk_y() as usize;
    if !(0..16).contains(&index) {
      return Err(pos.err("Y coordinate is outside of chunk".into()));
    }
    if index >= self.sections.len() {
      self.sections.resize_with(index + 1, || None);
    }
    if self.sections[index].is_none() {
      self.sections[index] = Some(S::new());
    }
    match &mut self.sections[index] {
      Some(s) => s.set_block(Pos::new(pos.x(), pos.chunk_rel_y(), pos.z()), ty),
      None => unreachable!(),
    }
  }
  /// This fills the given region with the given block. See
  /// [`set_block`](Self::set_block) for details about the bounds of min and
  /// max.
  pub fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError> {
    let min_index = min.chunk_y() as usize;
    let max_index = max.chunk_y() as usize;
    if !(0..16).contains(&min_index) {
      return Err(min.err("Y coordinate is outside of chunk".into()));
    }
    if !(0..16).contains(&max_index) {
      return Err(max.err("Y coordinate is outside of chunk".into()));
    }
    if max_index < min_index {
      return Err(max.err("max is less than min".into()));
    }
    if max_index >= self.sections.len() {
      self.sections.resize_with(max_index + 1, || None);
    }
    for index in min_index..=max_index {
      if self.sections[index].is_none() {
        self.sections[index] = Some(S::new());
      }
      match &mut self.sections[index] {
        Some(s) => {
          let min = Pos::new(min.x(), cmp::max(min.y(), index as i32 * 16), min.z());
          let max = Pos::new(max.x(), cmp::min(max.y(), index as i32 * 16 + 15), max.z());
          s.fill(
            Pos::new(min.x(), min.chunk_rel_y(), min.z()),
            Pos::new(max.x(), max.chunk_rel_y(), max.z()),
            ty,
          )?;
        }
        None => unreachable!(),
      }
    }
    Ok(())
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  pub fn get_block(&self, pos: Pos) -> Result<u32, PosError> {
    let index = pos.chunk_y();
    if !(0..16).contains(&index) {
      return Err(pos.err("Y coordinate is outside of chunk".into()));
    }
    let index = index as usize;
    if index >= self.sections.len() || self.sections[index].is_none() {
      return Ok(0);
    }
    match &self.sections[index] {
      Some(s) => s.get_block(Pos::new(pos.x(), pos.chunk_rel_y(), pos.z())),
      None => unreachable!(),
    }
  }
  /// Returns true if there is a chunk section at the given Y coordinate. The Y
  /// coordinate is not a block coordinate, but a section index (Y = 0
  /// corresponds to the section from 0..15, Y = 1 corresponds to the section
  /// from 16..31, etc)
  pub fn has_section(&self, y: u32) -> bool {
    // None              => out of bounds            => false
    // Some(None)        => in array, but no section => false
    // Some(Some(chunk)) => in array, valid section  => true
    match self.sections.get(y as usize) {
      Some(Some(_)) => true,
      _ => false,
    }
  }
  /// Returns an iterator through all the internal chunk sections.
  pub fn sections(&self) -> impl ExactSizeIterator<Item = &Option<S>> { self.sections.iter() }

  /// Builds a heightmap of this chunk. Each long contains 9 bit entries, where
  /// each entry is the height of the world at the given X, Z coordinate. This
  /// is used within 1.14+ protocol data, and is a needlessly complicated format
  /// that you shouldn't waste any time thinking about.
  ///
  /// The only reason these are signed is because of NBT long arrays. In
  /// reality, they should be read as unsigned longs.
  pub fn build_heightmap_old(&self) -> Vec<i64> {
    let mut heightmap = vec![0; 256 * 9 / 64];
    let mut shift = 0;
    let mut index = 0;
    for z in 0..16 {
      for x in 0..16 {
        let v = self.height_at(Pos::new(x, 0, z)).unwrap() as u64;
        if shift > 64 - 9 {
          heightmap[index] |= (v.overflowing_shl(shift).0 & 0b111111111 << (64 - 9)) as i64;
          heightmap[index + 1] |= (v >> (64 - shift)) as i64;
        } else {
          heightmap[index] |= (v.overflowing_shl(shift).0) as i64;
        }
        shift += 9;
        if shift > 64 {
          shift -= 64;
          index += 1;
        }
      }
    }
    heightmap
  }
  /// Builds a heightmap of this chunk. Each long contains 9 bit entries, where
  /// each entry is the height of the world at the given X, Z coordinate. This
  /// is used within 1.16.5+ protocol data, and is a needlessly complicated
  /// format that you shouldn't waste any time thinking about.
  ///
  /// The reason this is a different function is because of compacted long
  /// arrays. In 1.16, they stopped making entries overlap two longs. This makes
  /// it easier to serialize/deserialize, but uses slightly more storage.
  ///
  /// The only reason these are signed is because of NBT long arrays. In
  /// reality, they should be read as unsigned longs.
  pub fn build_heightmap_new(&self) -> Vec<i64> {
    let mut heightmap = vec![0; (256.0 / (64 / 9) as f32).ceil() as usize];
    let mut index = 0;
    let mut shift = 0;
    for z in 0..16 {
      for x in 0..16 {
        let v = self.height_at(Pos::new(x, 0, z)).unwrap() as u64;
        heightmap[index] |= (v.overflowing_shl(shift).0) as i64;
        shift += 9;
        if shift > 64 {
          shift = 0; // Important! We just subtract 64 in the other one
          index += 1;
        }
      }
    }
    heightmap
  }
  /// Returns the world height at the given position. This is a simple loop, and
  /// should be avoided.
  pub fn height_at(&self, pos: Pos) -> Result<i32, PosError> {
    let max_y = self.sections().len() * 16;
    for y in (0..max_y).rev() {
      // This is correct; it is not a transparent check, just an air check.
      if self.get_block(pos.with_y(y as i32))? != 0 {
        return Ok(y as i32);
      }
    }
    Ok(0)
  }
}

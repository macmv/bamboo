pub mod fixed;
mod light;
pub mod paletted;
mod section;

pub use light::{BlockLight, LightChunk, SkyLight};
pub use section::Section;

use crate::math::{PosError, RelPos};
use std::cmp;

/// A chunk column. This is not `Clone`, because that would mean duplicating an
/// entire chunk, which you probably don't want to do.
///
/// Note that this should have a function called `duplicate` which does the same
/// thing as clone, but I have not implemented it, as I have never needed to
/// clone a chunk.
///
/// If you want to create a cross-versioned chunk, use `MultiChunk` (in
/// `bb_server`) instead.
pub struct Chunk<S: Section> {
  sections: Vec<Option<S>>,
  max_bpe:  u8,
}

impl<S: Section> Chunk<S> {
  /// Creates an empty chunk, that can be resized to any height.
  pub fn new(max_bpe: u8) -> Self { Chunk { sections: Vec::new(), max_bpe } }
  /// Creates a chunk from the given sections list.
  pub fn from_sections(sections: Vec<Option<S>>, max_bpe: u8) -> Self {
    Chunk { sections, max_bpe }
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  pub fn set_block(&mut self, pos: RelPos, ty: u32) -> Result<(), PosError> {
    if pos.y() < 0 {
      return Err(pos.err("Y is negative".into()));
    }
    let index = pos.chunk_y() as usize;
    if index >= self.sections.len() {
      self.sections.resize_with(index + 1, || None);
    }
    if self.sections[index].is_none() {
      self.sections[index] = Some(S::new(self.max_bpe));
    }
    match &mut self.sections[index] {
      Some(s) => {
        s.set_block(pos.section_rel(), ty);
        Ok(())
      }
      None => unreachable!(),
    }
  }
  /// This fills the given region with the given block. See
  /// [`set_block`](Self::set_block) for details about the bounds of min and
  /// max.
  pub fn fill(&mut self, min: RelPos, max: RelPos, ty: u32) -> Result<(), PosError> {
    if min.y() < 0 {
      return Err(min.err("Y is negative".into()));
    }
    if max.y() < 0 {
      return Err(max.err("Y is negative".into()));
    }
    let min_index = min.chunk_y() as usize;
    let max_index = max.chunk_y() as usize;
    if max_index < min_index {
      return Err(max.err("max is less than min".into()));
    }
    if max_index >= self.sections.len() {
      self.sections.resize_with(max_index + 1, || None);
    }
    for index in min_index..=max_index {
      if self.sections[index].is_none() {
        self.sections[index] = Some(S::new(self.max_bpe));
      }
      match &mut self.sections[index] {
        Some(s) => {
          let min = RelPos::new(min.x(), cmp::max(min.y(), index as i32 * 16), min.z());
          let max = RelPos::new(max.x(), cmp::min(max.y(), index as i32 * 16 + 15), max.z());
          s.fill(min.section_rel(), max.section_rel(), ty);
        }
        None => unreachable!(),
      }
    }
    Ok(())
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  pub fn get_block(&self, pos: RelPos) -> Result<u32, PosError> {
    if pos.y() < 0 {
      return Err(pos.err("Y is negative".into()));
    }
    let index = pos.chunk_y() as usize;
    if index >= self.sections.len() || self.sections[index].is_none() {
      // This assumes air is `0`.
      return Ok(0);
    }
    match &self.sections[index] {
      Some(s) => Ok(s.get_block(pos.section_rel())),
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
    matches!(self.sections.get(y as usize), Some(Some(_)))
  }
  /// Returns an iterator through all the internal chunk sections.
  pub fn sections(&self) -> impl ExactSizeIterator<Item = &Option<S>> { self.sections.iter() }

  /// Returns the section at the given index. If it doesn't exist, this will
  /// create an empty section.
  pub fn section_mut(&mut self, y: u32) -> &mut S {
    let index = y as usize;
    if index >= self.sections.len() {
      self.sections.resize_with(index + 1, || None);
    }
    if self.sections[index].is_none() {
      self.sections[index] = Some(S::new(self.max_bpe));
    }
    match &mut self.sections[index] {
      Some(s) => s,
      None => unreachable!(),
    }
  }

  /// Clears the section at the given Y coordinate. If the chunk is at the top,
  /// the internal chunk list will shrink. This is more effective than calling
  /// `section_mut.fill(<air>)`
  pub fn clear_section(&mut self, y: u32) {
    let index = y as usize;
    if index < self.sections.len() {
      // Clear the chunk.
      self.sections[index] = None;
      // Truncate the sections list.
      while let Some(last) = self.sections.last() {
        if last.is_none() {
          self.sections.pop();
        } else {
          break;
        }
      }
    }
  }

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
        let v = self.height_at(RelPos::new(x, 0, z)).unwrap() as u64;
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
        let v = self.height_at(RelPos::new(x, 0, z)).unwrap() as u64;
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
  pub fn height_at(&self, pos: RelPos) -> Result<i32, PosError> {
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

pub mod fixed;
pub mod paletted;
mod section;

use std::{cmp, collections::HashMap};

pub use section::Section;

use crate::{
  math::{Pos, PosError},
  proto,
};

#[derive(Debug, Clone, Copy)]
pub enum ChunkKind {
  Fixed,
  Paletted,
}

/// A chunk column. This is not clone, because that would mean duplicating an
/// entire chunk, which you probably don't want to do. If you do need to clone a
/// chunk, use [`Chunk::duplicate()`].
///
/// If you want to create a cross-versioned chunk, use [`MultiChunk`] instead.
pub struct Chunk {
  sections: Vec<Option<Box<dyn Section + Send>>>,
  kind:     ChunkKind,
}

impl Chunk {
  pub fn new(kind: ChunkKind) -> Self {
    Chunk { sections: Vec::new(), kind }
  }
  /// Returns the kind of chunk this is. For 1.8 chunks, this will be `Fixed`.
  /// For any other chunk, this will be `Paletted`.
  pub fn kind(&self) -> ChunkKind {
    self.kind
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
      self.sections[index] = Some(match &self.kind {
        ChunkKind::Paletted => Box::new(paletted::Section::new()),
        ChunkKind::Fixed => Box::new(fixed::Section::new()),
      });
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
        self.sections[index] = Some(match &self.kind {
          ChunkKind::Paletted => Box::new(paletted::Section::new()),
          ChunkKind::Fixed => Box::new(fixed::Section::new()),
        });
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
      Some(s) => s.get_block(pos),
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
  /// Generates a protobuf containing all of the chunk data. X and Z will both
  /// be 0.
  pub fn to_latest_proto(&self) -> proto::Chunk {
    let mut sections = HashMap::new();
    for (i, s) in self.sections.iter().enumerate() {
      match s {
        Some(s) => {
          sections.insert(i as i32, s.to_latest_proto());
        }
        None => {}
      }
    }
    proto::Chunk { sections, ..Default::default() }
  }
  /// Generates a protobuf containing all of the chunk data. X and Z will both
  /// be 0. This will call the given function for every block id it encounters.
  pub fn to_old_proto<F>(&self, f: F) -> proto::Chunk
  where
    F: Fn(u32) -> u32,
  {
    let mut sections = HashMap::new();
    for (i, s) in self.sections.iter().enumerate() {
      match s {
        Some(s) => {
          sections.insert(i as i32, s.to_old_proto(&f));
        }
        None => {}
      }
    }
    proto::Chunk { sections, ..Default::default() }
  }
  /// Generates a chunk from the given protobuf. The X and Z values will be
  /// ignored.
  pub fn from_latest_proto(pb: proto::Chunk, kind: ChunkKind) -> Self {
    let mut chunk = Chunk::new(kind);
    for (y, section) in pb.sections {
      // pb.sections is a HashMap, so the order is random
      if y as usize >= chunk.sections.len() {
        chunk.sections.resize_with(y as usize + 1, || None);
      }
      chunk.sections[y as usize] = Some(match kind {
        ChunkKind::Fixed => fixed::Section::from_latest_proto(section),
        ChunkKind::Paletted => paletted::Section::from_latest_proto(section),
      });
    }
    chunk
  }
  /// Generates a chunk from the given protobuf. The X and Z values will be
  /// ignored. The given function `f` will be called for block id that this
  /// function encounters.
  pub fn from_old_proto<F>(pb: proto::Chunk, kind: ChunkKind, f: F) -> Self
  where
    F: Fn(u32) -> u32,
  {
    let mut chunk = Chunk::new(kind);
    for (y, section) in pb.sections {
      // pb.sections is a HashMap, so the order is random
      if y as usize >= chunk.sections.len() {
        chunk.sections.resize_with(y as usize + 1, || None);
      }
      chunk.sections[y as usize] = Some(match kind {
        ChunkKind::Fixed => fixed::Section::from_old_proto(section, &f),
        ChunkKind::Paletted => paletted::Section::from_old_proto(section, &f),
      });
    }
    chunk
  }
}

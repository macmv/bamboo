mod fixed;
mod multi;
mod paletted;
mod section;

pub use multi::MultiChunk;

use std::collections::HashMap;

use crate::block;
use section::Section;

use common::{
  math::{Pos, PosError},
  proto,
  version::BlockVersion,
};

/// A chunk column. This is not clone, because that would mean duplicating an
/// entire chunk, which you probably don't want to do. If you do need to clone a
/// chunk, use [`Chunk::duplicate()`].
///
/// If you want to create a cross-versioned chunk, use [`MultiChunk`] instead.
pub struct Chunk {
  sections: Vec<Option<Box<dyn Section + Send>>>,
  ver:      BlockVersion,
}

impl Chunk {
  pub fn new(ver: BlockVersion) -> Self {
    Chunk { sections: Vec::new(), ver }
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn set_block(&mut self, pos: Pos, ty: &block::Type) -> Result<(), PosError> {
    let index = pos.chunk_y();
    if !(0..16).contains(&index) {
      return Err(pos.err("Y coordinate is outside of chunk".into()));
    }
    let index = index as usize;
    if index >= self.sections.len() {
      self.sections.resize_with(index + 1, || None);
    }
    if self.sections[index].is_none() {
      self.sections[index] = Some(if self.ver > BlockVersion::V1_8 {
        fixed::Section::new()
      } else {
        paletted::Section::new()
      })
    }
    match &mut self.sections[index] {
      Some(s) => s.set_block(pos, ty),
      None => unreachable!(),
    }
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn get_block(&self, pos: Pos) -> Result<block::Type, PosError> {
    let index = pos.chunk_y();
    if !(0..16).contains(&index) {
      return Err(pos.err("Y coordinate is outside of chunk".into()));
    }
    let index = index as usize;
    if index >= self.sections.len() || self.sections[index].is_none() {
      return Ok(block::Type::air());
    }
    match &self.sections[index] {
      Some(s) => s.get_block(pos),
      None => unreachable!(),
    }
  }
  /// Generates a protobuf containing all of the chunk data. X and Z will both
  /// be 0.
  pub fn to_proto(&self) -> proto::Chunk {
    let mut sections = HashMap::new();
    for (i, s) in self.sections.iter().enumerate() {
      match s {
        Some(s) => {
          sections.insert(i as i32, s.to_proto());
        }
        None => {}
      }
    }
    proto::Chunk { sections, ..Default::default() }
  }
}

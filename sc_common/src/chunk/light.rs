use super::{Chunk, Section};
use crate::math::Pos;

pub struct SkyLightChunk {
  sections: Vec<Option<LightSection>>,
}
pub struct BlockLightChunk {
  sections: Vec<Option<LightSection>>,
}

pub struct LightSection {
  /// 2048 bytes, each representing 2 blocks.
  data: Vec<u8>,
}

impl SkyLightChunk {
  pub fn new() -> Self { SkyLightChunk { sections: vec![] } }

  /// Should be called whenever a block is updated.
  pub fn update<S: Section>(&mut self, chunk: &Chunk<S>, pos: Pos) {
    if pos != pos.chunk_rel() {
      panic!("cannot update sky light chunk with position outside of chunk: {}", pos);
    }
    // TODO
  }
}

impl BlockLightChunk {
  pub fn new() -> Self { BlockLightChunk { sections: vec![] } }

  /// Should be called whenever a block is updated.
  pub fn update<S: Section>(&mut self, chunk: &Chunk<S>, pos: Pos) {
    if pos != pos.chunk_rel() {
      panic!("cannot update block light chunk with position outside of chunk: {}", pos);
    }
    // TODO
  }
  pub fn add_light_source<S: Section>(&mut self, chunk: &Chunk<S>, pos: Pos, level: u8) {
    if level >= 16 {
      panic!("light level cannot be above 15: {}", level);
    }
    self.get_section_mut(pos.chunk_y() as usize).set(pos.chunk_section_rel(), level);
    // TODO
  }

  pub fn get_section_mut(&mut self, idx: usize) -> &mut LightSection {
    if idx >= self.sections.len() {
      self.sections.resize_with(idx + 1, || None);
    }
    if self.sections[idx].is_none() {
      self.sections[idx] = Some(LightSection::new());
    }
    self.sections.get_mut(idx).unwrap().as_mut().unwrap()
  }
}

impl LightSection {
  pub fn new() -> Self { LightSection { data: vec![0; 2048] } }
  /// Gets the light value in the given block position.
  ///
  /// # Panics
  ///
  /// If any of the position axis are outside of 0.16.
  pub fn get(&self, pos: Pos) -> u8 {
    if pos != pos.chunk_section_rel() {
      panic!("cannot get light level for chunk outside of section: {:?}", pos);
    }
    // SAFETY: We just garunteed that this is a valid position
    unsafe { self.get_unchecked(pos) }
  }

  /// Sets the light value in the given block position.
  ///
  /// # Panics
  ///
  /// If the light level is outside of 0..16, or if any of the position axis are
  /// outside of 0.16.
  pub fn set(&mut self, pos: Pos, level: u8) {
    if pos != pos.chunk_section_rel() {
      panic!("cannot get light level for chunk outside of section: {:?}", pos);
    }
    if level >= 16 {
      panic!("light level cannot be above 15: {}", level);
    }
    // SAFETY: We just garunteed that this is a valid position and level
    unsafe { self.set_unchecked(pos, level) }
  }

  /// Gets the light value in the given block position.
  ///
  /// # Safety
  ///
  /// The given position must be within 0..16 on all axis.
  pub unsafe fn get_unchecked(&self, pos: Pos) -> u8 {
    let idx = (pos.x() as usize) << 8 | (pos.y() as usize) << 4 | (pos.z() as usize);
    self.data.get_unchecked(idx / 2) >> (4 * idx & 1)
  }

  /// Sets the light value in the given block position.
  ///
  /// # Safety
  ///
  /// The light level must be within 0..16, and then given position must be
  /// within 0..16 on all axis.
  pub unsafe fn set_unchecked(&mut self, pos: Pos, level: u8) {
    let idx = (pos.x() as usize) << 8 | (pos.y() as usize) << 4 | (pos.z() as usize);
    *self.data.get_unchecked_mut(idx / 2) = level << (4 * idx & 1);
  }
}

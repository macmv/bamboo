use super::{Chunk, Section};
use crate::math::{RelPos, SectionRelPos};
use bb_macros::Transfer;
use std::{fmt, marker::PhantomData};

pub trait LightPropagator {
  fn initial_level() -> u8;
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: RelPos,
  );
  fn propagate_all<P: LightPropagator, S: Section>(light: &mut LightChunk<P>, chunk: &Chunk<S>);
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightChunk<P: LightPropagator> {
  pub data: LightChunkData,
  marker:   PhantomData<P>,
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub struct LightChunkData {
  #[id = 0]
  sections: Vec<Option<LightSection>>,
}

#[derive(Transfer, Clone, PartialEq)]
pub struct LightSection {
  // 2048 bytes, each representing 2 blocks.
  #[id = 0]
  data: Vec<u8>,
}

impl Default for LightChunkData {
  fn default() -> Self { LightChunkData::new() }
}

impl<P: LightPropagator> Default for LightChunk<P> {
  fn default() -> Self { LightChunk::new() }
}

impl fmt::Debug for LightSection {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    writeln!(f, "LightChunk([")?;
    writeln!(f, "each square is on one z plane")?;
    writeln!(f, "z 0..8:")?;
    for y in 0..16 {
      for z in 0..8 {
        for x in 0..16 {
          let v = self.get(SectionRelPos::new(x, y, z));
          if v == 0 {
            write!(f, ".")?;
          } else {
            write!(f, "{v:x}")?;
          }
        }
        write!(f, " ")?;
      }
      writeln!(f)?;
    }
    writeln!(f, "z 8..16:")?;
    for y in 0..16 {
      for z in 8..16 {
        for x in 0..16 {
          let v = self.get(SectionRelPos::new(x, y, z));
          if v == 0 {
            write!(f, ".")?;
          } else {
            write!(f, "{v:x}")?;
          }
        }
        write!(f, " ")?;
      }
      writeln!(f)?;
    }
    writeln!(f, "])")
  }
}

impl LightChunkData {
  pub fn new() -> Self { LightChunkData { sections: vec![] } }

  pub fn get_section_opt(&self, idx: usize) -> Option<&LightSection> {
    match self.sections.get(idx) {
      Some(Some(section)) => Some(section),
      _ => None,
    }
  }

  pub fn get_section_mut(&mut self, idx: usize) -> &mut LightSection {
    if idx >= self.sections.len() {
      self.sections.resize_with(idx + 1, || None);
    }
    if self.sections[idx].is_none() {
      self.sections[idx] = Some(LightSection::new(0));
    }
    self.sections.get_mut(idx).unwrap().as_mut().unwrap()
  }
  pub fn get_section(&mut self, idx: usize) -> &LightSection {
    if idx >= self.sections.len() {
      self.sections.resize_with(idx + 1, || None);
    }
    if self.sections[idx].is_none() {
      self.sections[idx] = Some(LightSection::new(0));
    }
    self.sections.get(idx).unwrap().as_ref().unwrap()
  }
}

impl<P: LightPropagator> LightChunk<P> {
  pub fn new() -> Self {
    LightChunk { data: LightChunkData::new(), marker: PhantomData::default() }
  }

  /// Should be called whenever a block is updated.
  pub fn update<S: Section>(&mut self, chunk: &Chunk<S>, pos: RelPos) {
    P::propagate(self, chunk, pos)
  }

  /// Should be called whenever a large portion of the chunk is changed.
  pub fn update_all<S: Section>(&mut self, chunk: &Chunk<S>) {
    for (y, _) in chunk.sections().enumerate() {
      self.get_section(y);
    }
    P::propagate_all(self, chunk)
  }

  pub fn sections(&self) -> &[Option<LightSection>] { &self.data.sections }

  pub fn get_section_mut(&mut self, idx: usize) -> &mut LightSection {
    let section = self.data.get_section_mut(idx);
    if P::initial_level() != 0 {
      section.set_all(P::initial_level());
    }
    section
  }
  pub fn get_section(&mut self, idx: usize) -> &LightSection {
    let section = self.data.get_section_mut(idx);
    if P::initial_level() != 0 {
      section.set_all(P::initial_level());
    }
    section
  }

  pub fn get_light(&mut self, pos: RelPos) -> u8 {
    self.get_section(pos.chunk_y() as usize).get(pos.section_rel())
  }
  pub fn set_light(&mut self, pos: RelPos, level: u8) {
    self.get_section_mut(pos.chunk_y() as usize).set(pos.section_rel(), level)
  }
}

impl LightSection {
  pub fn new(level: u8) -> Self { LightSection { data: vec![level | (level << 4); 2048] } }
  /// Gets the light value in the given block position.
  pub fn get(&self, pos: SectionRelPos) -> u8 {
    // SAFETY: `pos` is garunteed to be within 0..16
    unsafe {
      let idx = (pos.x() as usize) << 8 | (pos.y() as usize) << 4 | (pos.z() as usize);
      (self.data.get_unchecked(idx / 2) >> (4 * (idx % 2))) & 0x0f
    }
  }

  /// Sets the light value for the entire chunk.
  ///
  /// # Panics
  ///
  /// If the light level is outside of 0..16.
  pub fn set_all(&mut self, level: u8) {
    if level >= 16 {
      panic!("light level cannot be above 15: {}", level);
    }
    let value = (level << 4) | level;
    for elem in &mut self.data {
      *elem = value;
    }
  }

  /// Sets the light value in the given block position.
  ///
  /// # Panics
  ///
  /// If the light level is outside of 0..16, or if any of the position axis are
  /// outside of 0.16.
  pub fn set(&mut self, pos: SectionRelPos, level: u8) {
    if level >= 16 {
      panic!("light level cannot be above 15: {}", level);
    }
    // SAFETY: We just garunteed that this is a valid level, and `pos` is going to
    // be within 0..16 on all axis
    unsafe {
      let idx = (pos.x() as usize) << 8 | (pos.y() as usize) << 4 | (pos.z() as usize);
      *self.data.get_unchecked_mut(idx / 2) &= !(0xf << (4 * (idx % 2)));
      *self.data.get_unchecked_mut(idx / 2) |= level << (4 * (idx % 2));
    }
  }

  /// Returns the internal lighting data for this section. Can be sent directly
  /// to all clients.
  pub fn data(&self) -> &[u8] { &self.data }
}

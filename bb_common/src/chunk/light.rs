use super::{Chunk, Section};
use crate::math::{ChunkPos, Pos};
use bb_macros::Transfer;
use std::marker::PhantomData;

pub trait LightPropagator {
  fn initial_level() -> u8;
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: Pos,
  );
  fn propagate_all<P: LightPropagator, S: Section>(light: &mut LightChunk<P>, chunk: &Chunk<S>);
}

#[derive(Transfer, Debug, Clone)]
pub struct LightChunk<P: LightPropagator> {
  #[id = 0]
  sections: Vec<Option<LightSection>>,
  #[id = 1]
  marker:   PhantomData<P>,
}

#[derive(Transfer, Debug, Clone)]
pub struct LightSection {
  // 2048 bytes, each representing 2 blocks.
  #[id = 0]
  data: Vec<u8>,
}

impl<P: LightPropagator> Default for LightChunk<P> {
  fn default() -> Self { LightChunk::new() }
}

impl<P: LightPropagator> LightChunk<P> {
  pub fn new() -> Self { LightChunk { sections: vec![], marker: PhantomData::default() } }

  /// Should be called whenever a block is updated.
  pub fn update<S: Section>(&mut self, chunk: &Chunk<S>, pos: Pos) {
    if pos != pos.chunk_rel() {
      panic!("cannot update sky light chunk with position outside of chunk: {}", pos);
    }
    P::propagate(self, chunk, pos)
  }

  /// Should be called whenever a large portion of the chunk is changed.
  pub fn update_all<S: Section>(&mut self, chunk: &Chunk<S>) {
    for (y, _) in chunk.sections().enumerate() {
      self.get_section(y);
    }
    P::propagate_all(self, chunk)
  }

  pub fn sections(&self) -> &[Option<LightSection>] { &self.sections }

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
      self.sections[idx] = Some(LightSection::new(P::initial_level()));
    }
    self.sections.get_mut(idx).unwrap().as_mut().unwrap()
  }
  pub fn get_section(&mut self, idx: usize) -> &LightSection {
    if idx >= self.sections.len() {
      self.sections.resize_with(idx + 1, || None);
    }
    if self.sections[idx].is_none() {
      self.sections[idx] = Some(LightSection::new(P::initial_level()));
    }
    self.sections.get(idx).unwrap().as_ref().unwrap()
  }

  pub fn get_light(&mut self, pos: Pos) -> u8 {
    self.get_section(pos.chunk_y() as usize).get(pos.chunk_section_rel())
  }
  pub fn set_light(&mut self, pos: Pos, level: u8) {
    self.get_section_mut(pos.chunk_y() as usize).set(pos.chunk_section_rel(), level)
  }
}

/// Marker trait, which will propagate block light information.
#[derive(Debug, Clone)]
pub struct BlockLight {}
/// Marker trait, which will propagate sky light information.
#[derive(Debug, Clone)]
pub struct SkyLight {}

impl LightPropagator for BlockLight {
  fn initial_level() -> u8 { 0 }
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: Pos,
  ) {
    let directions = [
      Pos::new(0, 1, 0),
      Pos::new(0, -1, 0),
      Pos::new(1, 0, 0),
      Pos::new(-1, 0, 0),
      Pos::new(0, 0, 1),
      Pos::new(0, 0, -1),
    ];
    let level = light.get_light(pos);
    let mut queue = vec![(pos, level)];
    let mut other_queue = vec![];
    while !queue.is_empty() {
      for &(source, level) in &queue {
        if level == 0 {
          continue;
        }
        for dir in directions {
          let new_pos = source + dir;
          if new_pos.y() < 0 || new_pos.y() > 255 || new_pos.chunk() != ChunkPos::new(0, 0) {
            continue;
          }
          if chunk.get_block(new_pos).unwrap() == 0 && light.get_light(new_pos) < level - 1 {
            light.set_light(new_pos, level - 1);
            other_queue.push((new_pos, level - 1));
          }
        }
      }
      queue.clear();
      std::mem::swap(&mut queue, &mut other_queue);
    }
  }
  fn propagate_all<P: LightPropagator, S: Section>(light: &mut LightChunk<P>, chunk: &Chunk<S>) {
    let _ = (light, chunk);
    // TODO
  }
}

impl LightPropagator for SkyLight {
  fn initial_level() -> u8 { 15 }
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: Pos,
  ) {
    let directions = [
      Pos::new(0, 1, 0),
      Pos::new(0, -1, 0),
      Pos::new(1, 0, 0),
      Pos::new(-1, 0, 0),
      Pos::new(0, 0, 1),
      Pos::new(0, 0, -1),
    ];
    let level = light.get_light(pos);
    let mut queue = vec![(pos, level)];
    let mut other_queue = vec![];
    while !queue.is_empty() {
      for &(source, level) in &queue {
        if level == 0 {
          continue;
        }
        for dir in directions {
          let new_pos = source + dir;
          if new_pos.y() < 0 || new_pos.y() > 255 || new_pos.chunk() != ChunkPos::new(0, 0) {
            continue;
          }
          if chunk.get_block(new_pos).unwrap() == 0 {
            if dir == Pos::new(0, -1, 0) {
              other_queue.push((new_pos, level));
            } else if light.get_light(new_pos) < level - 1 {
              light.set_light(new_pos, level - 1);
              other_queue.push((new_pos, level));
            }
          }
        }
      }
      queue.clear();
      std::mem::swap(&mut queue, &mut other_queue);
    }
  }
  fn propagate_all<P: LightPropagator, S: Section>(light: &mut LightChunk<P>, chunk: &Chunk<S>) {
    let _ = (light, chunk);
    // TODO
  }
}

impl LightSection {
  pub fn new(level: u8) -> Self { LightSection { data: vec![level | (level << 4); 2048] } }
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
    self.data.get_unchecked(idx / 2) >> (4 * (idx % 2))
  }

  /// Sets the light value in the given block position.
  ///
  /// # Safety
  ///
  /// The light level must be within 0..16, and then given position must be
  /// within 0..16 on all axis.
  pub unsafe fn set_unchecked(&mut self, pos: Pos, level: u8) {
    let idx = (pos.x() as usize) << 8 | (pos.y() as usize) << 4 | (pos.z() as usize);
    *self.data.get_unchecked_mut(idx / 2) = level << (4 * (idx % 2));
  }

  /// Returns the internal lighting data for this section. Can be sent directly
  /// to all clients.
  pub fn data(&self) -> &[u8] { &self.data }
}

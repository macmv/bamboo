use super::{Chunk, Section};
use crate::math::{ColRelPos, Face, RelPos};
use bb_macros::Transfer;
use std::{fmt, marker::PhantomData};

#[cfg(test)]
mod test;

pub trait LightPropagator {
  fn initial_level() -> u8;
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: ColRelPos,
  );
  fn propagate_all<P: LightPropagator, S: Section>(light: &mut LightChunk<P>, chunk: &Chunk<S>);
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub struct LightChunk<P: LightPropagator> {
  #[id = 0]
  sections: Vec<Option<LightSection>>,
  #[id = 1]
  marker:   PhantomData<P>,
}

#[derive(Transfer, Clone, PartialEq)]
pub struct LightSection {
  // 2048 bytes, each representing 2 blocks.
  #[id = 0]
  data: Vec<u8>,
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
          let v = self.get(RelPos::new(x, y, z));
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
          let v = self.get(RelPos::new(x, y, z));
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

impl<P: LightPropagator> LightChunk<P> {
  pub fn new() -> Self { LightChunk { sections: vec![], marker: PhantomData::default() } }

  /// Should be called whenever a block is updated.
  pub fn update<S: Section>(&mut self, chunk: &Chunk<S>, pos: ColRelPos) {
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

  pub fn get_light(&mut self, pos: ColRelPos) -> u8 {
    self.get_section(pos.chunk_y() as usize).get(pos.chunk_rel())
  }
  pub fn set_light(&mut self, pos: ColRelPos, level: u8) {
    self.get_section_mut(pos.chunk_y() as usize).set(pos.chunk_rel(), level)
  }
}

/// Marker trait, which will propagate block light information.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockLight {}
/// Marker trait, which will propagate sky light information.
#[derive(Debug, Clone, PartialEq)]
pub struct SkyLight {}

impl LightPropagator for BlockLight {
  fn initial_level() -> u8 { 0 }
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: ColRelPos,
  ) {
    let directions = [Face::Up, Face::Down, Face::North, Face::South, Face::East, Face::West];
    let level = light.get_light(pos);
    let mut queue = vec![(pos, level)];
    let mut other_queue = vec![];
    while !queue.is_empty() {
      for &(source, level) in &queue {
        if level == 0 {
          continue;
        }
        for dir in directions {
          let new_pos = match source.checked_add(dir) {
            Some(p) => p,
            None => continue,
          };
          if new_pos.y() < 0 || new_pos.y() > 255 {
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
    pos: ColRelPos,
  ) {
    let directions = [Face::Up, Face::Down, Face::North, Face::South, Face::East, Face::West];
    let level = light.get_light(pos);
    let mut queue = vec![(pos, level)];
    let mut other_queue = vec![];
    while !queue.is_empty() {
      for &(source, level) in &queue {
        if level == 0 {
          continue;
        }
        for dir in directions {
          let new_pos = match source.checked_add(dir) {
            Some(p) => p,
            None => continue,
          };
          if new_pos.y() < 0 || new_pos.y() > 255 {
            continue;
          }
          if chunk.get_block(new_pos).unwrap() == 0 {
            if dir == Face::Down {
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
  pub fn get(&self, pos: RelPos) -> u8 {
    // SAFETY: `pos` is garunteed to be within 0..16
    unsafe {
      let idx = (pos.x() as usize) << 8 | (pos.y() as usize) << 4 | (pos.z() as usize);
      (self.data.get_unchecked(idx / 2) >> (4 * (idx % 2))) & 0x0f
    }
  }

  /// Sets the light value in the given block position.
  ///
  /// # Panics
  ///
  /// If the light level is outside of 0..16, or if any of the position axis are
  /// outside of 0.16.
  pub fn set(&mut self, pos: RelPos, level: u8) {
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

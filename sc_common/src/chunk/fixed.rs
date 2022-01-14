use super::section::Section as ChunkSection;
use crate::math::{Pos, PosError};

/// Only used for 1.8. This is a chunk section that does not contain a palette.
pub struct Section {
  data: [u16; 16 * 16 * 16],
}

impl Section {
  /// Returns the internal data of this section.
  pub fn data(&self) -> &[u16; 16 * 16 * 16] { &self.data }
  /// Sets the block at the given position within the internal block data.
  ///
  /// # Safety
  ///
  /// - pos must be within `Pos(0, 0, 0)..Pos(16, 16, 16)`.
  #[inline(always)]
  unsafe fn set_block_unchecked(&mut self, pos: Pos, ty: u32) {
    *self
      .data
      .get_unchecked_mut(pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize) =
      ty as u16;
  }
}

impl ChunkSection for Section {
  fn new() -> Self { Section { data: [0; 16 * 16 * 16] } }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    unsafe {
      // SAFETY: We just checked that x, y, and z are all within 0..16, so
      // the position passed to set_block_unchecked is safe
      self.set_block_unchecked(pos, ty);
    }
    Ok(())
  }
  fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError> {
    if min.x() >= 16 || min.x() < 0 || min.y() >= 16 || min.y() < 0 || min.z() >= 16 || min.z() < 0
    {
      return Err(min.err("expected min to be within 0 <= x, y, z < 16".into()));
    }
    if max.x() >= 16 || max.x() < 0 || max.y() >= 16 || max.y() < 0 || max.z() >= 16 || max.z() < 0
    {
      return Err(max.err("expected max to be within 0 <= x, y, z < 16".into()));
    }
    for y in min.y()..=max.y() {
      for z in min.z()..=max.z() {
        for x in min.x()..=max.x() {
          unsafe {
            // SAFETY: We just checked that min/max x, y, and z are all within 0..16, so x,
            // y, and z will all be within 0..16.
            self.set_block_unchecked(Pos::new(x, y, z), ty);
          }
        }
      }
    }
    Ok(())
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn get_block(&self, pos: Pos) -> Result<u32, PosError> {
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    unsafe {
      // SAFETY: We just checked pos, so this will always be valid
      Ok(u32::from(
        *self
          .data
          .get_unchecked(pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize),
      ))
    }
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> { Box::new(Section { data: self.data }) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn set_block_id() {
    let mut s = Section::new();
    s.set_block(Pos::new(1, 0, 0), 5).unwrap();
    s.set_block(Pos::new(0, 0, 1), 10).unwrap();
    s.set_block(Pos::new(0, 1, 0), 20).unwrap();
    let mut e = [0; 16 * 16 * 16];
    e[1] = 5;
    e[16] = 10;
    e[16 * 16] = 20;
    assert_eq!(s.data, e);

    assert!(s.set_block(Pos::new(0, 0, 16), 5).is_err());
  }

  #[test]
  fn get_block() {
    let mut s = Section::new();
    s.set_block(Pos::new(1, 0, 0), 5).unwrap();
    s.set_block(Pos::new(0, 1, 0), 10).unwrap();
    s.set_block(Pos::new(0, 0, 1), 20).unwrap();
    assert_eq!(s.get_block(Pos::new(1, 0, 0)).unwrap(), 5);
    assert_eq!(s.get_block(Pos::new(0, 1, 0)).unwrap(), 10);
    assert_eq!(s.get_block(Pos::new(0, 0, 1)).unwrap(), 20);
  }
}

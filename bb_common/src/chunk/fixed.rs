use super::section::Section as ChunkSection;
use crate::math::SectionRelPos;

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
  unsafe fn set_block_unchecked(&mut self, pos: SectionRelPos, ty: u32) {
    *self
      .data
      .get_unchecked_mut(pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize) =
      ty as u16;
  }
}

impl ChunkSection for Section {
  fn new(_: u8) -> Self { Section { data: [0; 16 * 16 * 16] } }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn set_block(&mut self, pos: SectionRelPos, ty: u32) {
    // SAFETY: By defintion, pos.{x,y,z} are all within 0..16, so
    // the position passed to set_block_unchecked is safe
    unsafe {
      self.set_block_unchecked(pos, ty);
    }
  }
  fn fill(&mut self, min: SectionRelPos, max: SectionRelPos, ty: u32) {
    for y in min.y()..=max.y() {
      for z in min.z()..=max.z() {
        for x in min.x()..=max.x() {
          unsafe {
            // SAFETY: By defintion, pos.{x,y,z} are all within 0..16, so
            // the position passed to set_block_unchecked is safe
            self.set_block_unchecked(SectionRelPos::new(x, y, z), ty);
          }
        }
      }
    }
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn get_block(&self, pos: SectionRelPos) -> u32 {
    unsafe {
      // SAFETY: By defintion, pos.{x,y,z} are all within 0..16, so
      // the position passed to set_block_unchecked is safe
      u32::from(
        *self
          .data
          .get_unchecked(pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize),
      )
    }
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> { Box::new(Section { data: self.data }) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn set_block_id() {
    let mut s = Section::new(0);
    s.set_block(SectionRelPos::new(1, 0, 0), 5);
    s.set_block(SectionRelPos::new(0, 0, 1), 10);
    s.set_block(SectionRelPos::new(0, 1, 0), 20);
    let mut e = [0; 16 * 16 * 16];
    e[1] = 5;
    e[16] = 10;
    e[16 * 16] = 20;
    assert_eq!(s.data, e);
  }

  #[test]
  #[should_panic]
  fn invalid_pos() { SectionRelPos::new(0, 0, 16); }

  #[test]
  fn get_block() {
    let mut s = Section::new(0);
    s.set_block(SectionRelPos::new(1, 0, 0), 5);
    s.set_block(SectionRelPos::new(0, 1, 0), 10);
    s.set_block(SectionRelPos::new(0, 0, 1), 20);
    assert_eq!(s.get_block(SectionRelPos::new(1, 0, 0)), 5);
    assert_eq!(s.get_block(SectionRelPos::new(0, 1, 0)), 10);
    assert_eq!(s.get_block(SectionRelPos::new(0, 0, 1)), 20);
  }
}

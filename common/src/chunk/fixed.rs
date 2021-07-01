use std::convert::{TryFrom, TryInto};

use super::section::Section as ChunkSection;
use crate::{
  math::{Pos, PosError},
  proto,
};

/// Only used for 1.8. This is a chunk section that does not contain a palette.
pub struct Section {
  data: [u16; 16 * 16 * 16],
}

impl Section {
  pub(super) fn new() -> Box<Self> {
    Box::new(Section { data: [0; 16 * 16 * 16] })
  }
  pub(super) fn from_latest_proto(pb: proto::chunk::Section) -> Box<Self> {
    assert_eq!(pb.data.len(), 4096 / 4, "chunk data is the wrong length");
    let mut section = Section { data: [0; 4096] };
    // Using map() and collect() would cause more allocations than this
    for (i, v) in pb.data.iter().enumerate() {
      section.data[i * 4 + 0] = (v >> 0) as u16;
      section.data[i * 4 + 1] = (v >> 16) as u16;
      section.data[i * 4 + 2] = (v >> 32) as u16;
      section.data[i * 4 + 3] = (v >> 48) as u16;
    }
    Box::new(section)
  }
  pub(super) fn from_old_proto(pb: proto::chunk::Section, f: &dyn Fn(u32) -> u32) -> Box<Self> {
    assert_eq!(pb.data.len(), 4096 / 4, "chunk data is the wrong length");
    let mut section = Section { data: [0; 4096] };
    // Using map() and collect() would cause more allocations than this
    for (i, v) in pb.data.iter().enumerate() {
      section.data[i * 4 + 0] = f(((v >> 0) as u16).into()).try_into().unwrap();
      section.data[i * 4 + 1] = f(((v >> 16) as u16).into()).try_into().unwrap();
      section.data[i * 4 + 2] = f(((v >> 32) as u16).into()).try_into().unwrap();
      section.data[i * 4 + 3] = f(((v >> 48) as u16).into()).try_into().unwrap();
    }
    Box::new(section)
  }
  /// Sets the block at the given position within the internal block data.
  ///
  /// # Safety
  ///
  /// - pos must be within `Pos(0, 0, 0)..Pos(16, 16, 16)`.
  unsafe fn set_block_unchecked(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    *self
      .data
      .get_unchecked_mut(pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize) =
      ty as u16;
    Ok(())
  }
}

impl ChunkSection for Section {
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    unsafe {
      // SAFETY: We just checked that x, y, and z are all within 0..16, so the
      // position passed to set_block_unchecked is safe
      self.set_block_unchecked(pos, ty)
    }
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
            self.set_block_unchecked(Pos::new(x, y, z), ty)?;
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
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section { data: self.data })
  }
  /// This is always called, because this type of chunk section is only used for
  /// one version.
  fn to_latest_proto(&self) -> proto::chunk::Section {
    let mut data = vec![0; self.data.len() / 4]; // 8 bytes per u64, 2 bytes per u16
    for (i, id) in self.data.iter().enumerate() {
      unsafe {
        // SAFETY: self.data.len() = 4096 (which is 8192 bytes), and data.len() = 2048
        // (8192 / sizeof(u64)). So, i / 4 is always within data
        let v = data.get_unchecked_mut(i / 4);
        *v |= u64::from(*id) << (i % 4 * 16);
      }
    }
    proto::chunk::Section { data, ..Default::default() }
  }
  /// Never called, as fixed chunks only are used on one version.
  fn to_old_proto(&self, f: &dyn Fn(u32) -> u32) -> proto::chunk::Section {
    let mut data = vec![0; self.data.len() / 4]; // 8 bytes per u64, 2 bytes per u16
    for (i, id) in self.data.iter().enumerate() {
      unsafe {
        // SAFETY: self.data.len() = 4096 (which is 8192 bytes), and data.len() = 2048
        // (8192 / sizeof(u64)). So, i / 4 is always within data
        let v = data.get_unchecked_mut(i / 4);
        *v |= u64::from(f(u32::from(*id))) << (i % 4 * 16);
      }
    }
    proto::chunk::Section { data, ..Default::default() }
  }
}

#[cfg(test)]
mod tests {
  extern crate test;

  use super::*;
  use test::Bencher;

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

  /// # Test results
  ///
  /// Opt level 3 did not change the results much. After using
  /// https://rust.godbolt.org/, I realized that without any optimizations,
  /// iterating over anything is very, very slow. So I am no longer going to
  /// work on optimizing the unsafe calls, as those checks will get compiled
  /// out any time speed actually matters.
  ///
  /// Optlevel:          0    |     1     |    2
  /// Fill:        ~200,000ns   ~78,000ns   ~5,000ns
  /// Fill manual: ~200,000ns   ~76,000ns   ~7,500ns

  #[bench]
  fn fill_manual(b: &mut Bencher) {
    let mut s = Section::new();
    let mut block = 0u8;
    b.iter(|| {
      for y in 0..16 {
        for z in 0..16 {
          for x in 0..16 {
            s.set_block(Pos::new(x, y, z), block.into()).unwrap();
          }
        }
      }
      block = block.wrapping_add(1);
    });
  }

  #[bench]
  fn fill(b: &mut Bencher) {
    let mut s = Section::new();
    let mut block = 0u8;
    b.iter(|| {
      s.fill(Pos::new(0, 0, 0), Pos::new(15, 15, 15), block.into()).unwrap();
      block = block.wrapping_add(1);
    });
  }
}

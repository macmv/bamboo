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
}

impl ChunkSection for Section {
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    #[cfg(debug_assertions)]
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    #[cfg(not(debug_assertions))]
    let v = ty as u16;
    #[cfg(debug_assertions)]
    let v = u16::try_from(ty).unwrap();
    self.data[pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize] = v;
    Ok(())
  }
  fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError> {
    for y in min.y()..=max.y() {
      for z in min.z()..=max.z() {
        for x in min.x()..=max.x() {
          self.set_block(Pos::new(x, y, z), ty)?;
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
    Ok(self.data[pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize].into())
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section { data: self.data })
  }
  /// This is always called, because this type of chunk section is only used for
  /// one version.
  fn to_latest_proto(&self) -> proto::chunk::Section {
    let mut data = vec![0; self.data.len() / 4]; // 8 bytes per u64, 2 bytes per u16
    for (i, id) in self.data.iter().enumerate() {
      data[i / 4] |= u64::from(*id) << (i % 4 * 16);
    }
    proto::chunk::Section { data, ..Default::default() }
  }
  /// Never called, as fixed chunks only are used on one version.
  fn to_old_proto(&self, f: &dyn Fn(u32) -> u32) -> proto::chunk::Section {
    let mut data = vec![0; self.data.len() / 4]; // 8 bytes per u64, 2 bytes per u16
    for (i, id) in self.data.iter().enumerate() {
      data[i / 4] |= u64::from(f(u32::from(*id))) << (i % 4 * 16);
    }
    proto::chunk::Section { data, ..Default::default() }
  }
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

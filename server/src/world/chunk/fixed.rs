use std::convert::TryFrom;

use super::section::Section as ChunkSection;
use common::{
  math::{Pos, PosError},
  proto,
};

pub struct Section {
  data: [u16; 16 * 16 * 16],
}

impl Section {
  pub(super) fn new() -> Box<Self> {
    Box::new(Section { data: [0; 16 * 16 * 16] })
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
    #[cfg(not(debug_assertions))]
    let v = ty as u16;
    #[cfg(debug_assertions)]
    let v = u16::try_from(ty).unwrap();
    self.data[pos.y() as usize * 16 * 16 + pos.z() as usize * 16 + pos.x() as usize] = v;
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
  fn to_latest_proto(&self) -> proto::chunk::Section {
    proto::chunk::Section::default()
  }
  fn to_old_proto(&self, f: &dyn Fn(u32) -> u32) -> proto::chunk::Section {
    proto::chunk::Section::default()
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

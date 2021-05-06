use super::section::Section as ChunkSection;
use crate::block;

use common::{
  math::{Pos, PosError},
  proto,
};

pub struct Section {
  data: [u16; 16 * 16 * 16],
}

impl Section {
  pub(super) fn new() -> Box<dyn ChunkSection + Send> {
    Box::new(Section { data: [0; 16 * 16 * 16] })
  }
  fn set_block_id(&mut self, p: Pos, id: u16) {
    self.data[p.y() as usize * 16 * 16 + p.z() as usize * 16 + p.x() as usize] = id;
  }
  fn get_block_id(&self, p: Pos) -> u16 {
    self.data[p.y() as usize * 16 * 16 + p.z() as usize * 16 + p.x() as usize]
  }
}

impl ChunkSection for Section {
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  #[cfg(debug_assertions)]
  fn set_block(&mut self, pos: Pos, ty: &block::Type) -> Result<(), PosError> {
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    self.set_block_id(pos, ty.id() as u16);
    Ok(())
  }
  #[cfg(not(debug_assertions))]
  fn set_block(&mut self, pos: Pos, ty: block::Type) -> Result<(), PosError> {
    self.set_block_id(pos, ty.id() as u16);
    Ok(())
  }
  /// This updates the internal data to contain a block at the given position.
  /// In release mode, the position is not checked. In any other mode, a
  /// PosError will be returned if any of the x, y, or z are outside of 0..16
  #[cfg(debug_assertions)]
  fn get_block(&self, pos: Pos) -> Result<block::Type, PosError> {
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    Ok(block::Type::from_id(self.get_block_id(pos) as u32))
  }
  #[cfg(not(debug_assertions))]
  fn get_block(&self, pos: Pos) -> Result<block::Type, PosError> {
    Ok(block::Type::from_id(self.get_block_id(pos) as u32))
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section { data: self.data.clone() })
  }
  fn to_proto(&self) -> proto::chunk::Section {
    proto::chunk::Section::default()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn set_block_id() {
    let mut s = Section::new();
    s.set_block_id(Pos::new(1, 0, 0), 5);
    s.set_block_id(Pos::new(0, 0, 1), 10);
    s.set_block_id(Pos::new(0, 1, 0), 20);
    let mut e = [0; 16 * 16 * 16];
    e[1] = 5;
    e[16] = 10;
    e[16 * 16] = 20;
    assert_eq!(s.data, e);
  }

  #[test]
  fn get_block_id() {
    let mut s = Section::new();
    s.set_block_id(Pos::new(1, 0, 0), 5);
    s.set_block_id(Pos::new(0, 1, 0), 10);
    s.set_block_id(Pos::new(0, 0, 1), 20);
    assert_eq!(s.get_block_id(Pos::new(1, 0, 0)), 5);
    assert_eq!(s.get_block_id(Pos::new(0, 1, 0)), 10);
    assert_eq!(s.get_block_id(Pos::new(0, 0, 1)), 20);
  }
}

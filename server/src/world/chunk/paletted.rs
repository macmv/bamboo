use super::section::Section as ChunkSection;

use common::{
  math::{Pos, PosError},
  proto,
};
use std::collections::HashMap;

pub struct Section {
  bits_per_block:  u8,
  data:            Vec<u64>,
  // Each index into palette is a palette id. The values are global ids.
  palette:         Vec<u32>,
  // This maps global ids to palette ids.
  reverse_palette: HashMap<u32, u32>,
}

impl Default for Section {
  fn default() -> Self {
    let mut reverse_palette = HashMap::new();
    reverse_palette.insert(0, 0);
    Section {
      bits_per_block: 4,
      // Number of blocks times bits per block divided by sizeof(u64)
      data: vec![0; 16 * 16 * 16 * 4 / 64],
      palette: vec![0],
      reverse_palette,
    }
  }
}

impl Section {
  pub(super) fn new() -> Box<Self> {
    Box::new(Self::default())
  }
  fn index(&self, pos: Pos) -> (usize, usize, usize) {
    let index = (pos.y() << 8 | pos.z() << 4 | pos.x()) as usize;
    let bpb = self.bits_per_block as usize;
    let first = index * bpb / 64;
    let second = (index + 1) * bpb / 64;
    let shift = index * bpb % 64;
    (first, second, shift)
  }
  /// Writes a single palette id into self.data.
  fn set_palette(&mut self, pos: Pos, id: u32) {
    let (first, second, shift) = self.index(pos);
  }
}

impl ChunkSection for Section {
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    if let Some(&palette_id) = self.reverse_palette.get(&ty) {
      self.set_palette(pos, palette_id);
    }
    Ok(())
  }
  fn get_block(&self, _pos: Pos) -> Result<u32, PosError> {
    Ok(0)
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section {
      bits_per_block:  self.bits_per_block,
      data:            self.data.clone(),
      palette:         self.palette.clone(),
      reverse_palette: self.reverse_palette.clone(),
    })
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
  fn test_index() {
    let s = Section::default();
    assert_eq!(s.index(Pos::new(0, 0, 0)), (0, 0, 0));
    assert_eq!(s.index(Pos::new(1, 0, 0)), (0, 0, 4));
    assert_eq!(s.index(Pos::new(2, 0, 0)), (0, 0, 8));
    assert_eq!(s.index(Pos::new(0, 0, 1)), (1, 1, 0));

    let s = Section { bits_per_block: 5, ..Default::default() };
    assert_eq!(s.index(Pos::new(0, 0, 0)), (0, 0, 0));
    assert_eq!(s.index(Pos::new(1, 0, 0)), (0, 0, 5));
    // The id will be split between two longs
    assert_eq!(s.index(Pos::new(12, 0, 0)), (0, 1, 60));
    assert_eq!(s.index(Pos::new(13, 0, 0)), (1, 1, 1));
  }
}

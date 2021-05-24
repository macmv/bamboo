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
    let bpb = self.bits_per_block as usize;
    if id >= 1 << bpb {
      panic!("passed invalid id {} (must be within 0..{})", id, 1 << bpb);
    }
    if first == second {
      // Clear the bits of the new id
      self.data[first] &= !(((1 << bpb) - 1) << shift);
      // Set the new id
      self.data[first] |= (id as u64) << shift;
    } else {
      let second_shift = 64 - shift;
      // Clear the bits of the new id
      self.data[first] &= !(((1 << bpb) - 1) << shift);
      self.data[second] &= !(((1 << bpb) - 1) >> second_shift);
      // Set the new id
      self.data[first] |= (id as u64) << shift;
      self.data[second] |= (id as u64) >> second_shift;
    }
  }
  /// This adds a new item to the palette. It will shift all block data, and
  /// extend bits per block (if needed). `ty` must not already be in the
  /// palette. Returns the new palette id.
  fn insert(&mut self, ty: u32) -> u32 {
    if self.palette.len() + 1 >= 1 << self.bits_per_block as usize {
      self.increase_bits_per_block();
    }
    let mut palette_id = self.palette.len() as u32;
    for (i, g) in self.palette.iter().enumerate() {
      if *g > ty {
        palette_id = (i - 1) as u32;
        break;
      }
    }
    self.palette.insert(palette_id as usize, ty);
    for (_, p) in self.reverse_palette.iter_mut() {
      if *p > palette_id {
        *p += 1;
      }
    }
    self.reverse_palette.insert(ty, palette_id);
    palette_id
  }
  /// Increases the bits per block by one. This will increase
  /// self.bits_per_block, and update the long array.
  fn increase_bits_per_block(&mut self) {
    let bpb = (self.bits_per_block + 1) as usize;
    let new_data = vec![0; 16 * 16 * 16 * bpb as usize / 64];
    let mut bit_index = 0;
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          bit_index += bpb;
          let first = bit_index / 64;
          let second = (bit_index + bpb) / 64;
          let shift = bit_index % 64;
          let id = self.get_palette(Pos::new(x, y, z));
          if first == second {
            // Clear the bits of the new id
            self.data[first] &= !(((1 << bpb) - 1) << shift);
            // Set the new id
            self.data[first] |= (id as u64) << shift;
          } else {
            let second_shift = 64 - shift;
            // Clear the bits of the new id
            self.data[first] &= !(((1 << bpb) - 1) << shift);
            self.data[second] &= !(((1 << bpb) - 1) >> second_shift);
            // Set the new id
            self.data[first] |= (id as u64) << shift;
            self.data[second] |= (id as u64) >> second_shift;
          }
        }
      }
    }
    self.data = new_data;
  }
}

impl ChunkSection for Section {
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    if let Some(&palette_id) = self.reverse_palette.get(&ty) {
      self.set_palette(pos, palette_id);
    } else {
      let palette_id = self.insert(ty);
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

  #[test]
  fn test_set_palette() {
    let mut s = Section::default();
    // Sanity check
    s.set_palette(Pos::new(0, 0, 0), 0xf);
    assert_eq!(s.data[0], 0xf);
    // Sanity check
    s.set_palette(Pos::new(2, 0, 0), 0xf);
    assert_eq!(s.data[0], 0xf0f);
    // Should work up to the edge of the long
    s.set_palette(Pos::new(15, 0, 0), 0xf);
    assert_eq!(s.data[0], 0xf000000000000f0f);
    // Clearing bits should work
    s.set_palette(Pos::new(15, 0, 0), 0x3);
    assert_eq!(s.data[0], 0x3000000000000f0f);

    let mut s = Section { bits_per_block: 5, ..Default::default() };
    // Sanity check
    s.set_palette(Pos::new(0, 0, 0), 0x1f);
    assert_eq!(s.data[0], 0x1f);
    // Sanity check
    s.set_palette(Pos::new(2, 0, 0), 0x1f);
    assert_eq!(s.data[0], 0x1f << 10 | 0x1f);
    // Should split the id correctly
    s.set_palette(Pos::new(12, 0, 0), 0x1f);
    assert_eq!(s.data[0], 0x1f << 60 | 0x1f << 10 | 0x1f);
    assert_eq!(s.data[1], 0x1f >> 4);
    s.set_palette(Pos::new(25, 0, 0), 0x1f);
    assert_eq!(s.data[1], 0x1f << 61 | 0x1f >> 4);
    assert_eq!(s.data[2], 0x1f >> 3);
    // Clearing bits should work
    s.set_palette(Pos::new(0, 0, 0), 0x3);
    assert_eq!(s.data[0], 0x1f << 60 | 0x1f << 10 | 0x03);
  }
}

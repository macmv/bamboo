use super::section::Section as ChunkSection;

use common::{
  math::{Pos, PosError},
  proto,
};
use std::{collections::HashMap, convert::TryFrom};

pub struct Section {
  bits_per_block:  u8,
  data:            Vec<u64>,
  // Each index into palette is a palette id. The values are global ids.
  palette:         Vec<u32>,
  // Each index is a palette id, and the sum of this array must always be 4096 (16 * 16 * 16).
  block_amounts:   Vec<u32>,
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
      block_amounts: vec![4096],
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
    let bit_index = index * bpb;
    let first = bit_index / 64;
    let second = (bit_index + bpb - 1) / 64;
    let shift = bit_index % 64;
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
  /// Returns the palette id at the given position. This only reads from
  /// `self.data`.
  fn get_palette(&self, pos: Pos) -> u32 {
    let (first, second, shift) = self.index(pos);
    let bpb = self.bits_per_block as usize;
    let val = if first == second {
      // Get the id from data
      self.data[first] >> shift & ((1 << bpb) - 1)
    } else {
      let second_shift = 64 - shift;
      // Get the id from the two values
      self.data[first] >> shift & ((1 << bpb) - 1)
        | self.data[second] << second_shift & ((1 << bpb) - 1)
    };

    #[cfg(not(debug_assertions))]
    let v = val as u32;
    #[cfg(debug_assertions)]
    let v = u32::try_from(val).unwrap();
    v
  }
  /// This adds a new item to the palette. It will shift all block data, and
  /// extend bits per block (if needed). It will also update the palettes, and
  /// shift the block amounts around. `ty` must not already be in the palette.
  /// Returns the new palette id.
  fn insert(&mut self, ty: u32) -> u32 {
    if self.palette.len() + 1 >= 1 << self.bits_per_block as usize {
      self.increase_bits_per_block();
    }
    let mut palette_id = self.palette.len() as u32;
    for (i, g) in self.palette.iter().enumerate() {
      if *g > ty {
        palette_id = i as u32;
        break;
      }
    }
    self.palette.insert(palette_id as usize, ty);
    // We add to this in set_block, not here
    self.block_amounts.insert(palette_id as usize, 0);
    for (_, p) in self.reverse_palette.iter_mut() {
      if *p >= palette_id {
        *p += 1;
      }
    }
    self.reverse_palette.insert(ty, palette_id);
    palette_id
  }
  /// This removes the given palette id from the palette. This includes
  /// modifying the block_amounts array. It will also decrease the bits per
  /// block if needed. `id` must be a valid index into the palette.
  fn remove(&mut self, id: u32) {
    // if self.palette.len() - 1 < 1 << (self.bits_per_block as usize - 1) {
    //   self.decrease_bits_per_block();
    // }
    let ty = self.palette[id as usize];
    self.palette.remove(id as usize);
    self.block_amounts.remove(id as usize);
    for (_, p) in self.reverse_palette.iter_mut() {
      if *p > id {
        *p -= 1;
      }
    }
    self.reverse_palette.remove(&ty);
  }
  /// Increases the bits per block by one. This will increase
  /// self.bits_per_block, and update the long array. It does not affect the
  /// palette at all.
  fn increase_bits_per_block(&mut self) {
    let bpb = (self.bits_per_block + 1) as usize;
    let mut new_data = vec![0; 16 * 16 * 16 * bpb / 64];
    let mut bit_index = 0;
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          let first = bit_index / 64;
          let second = (bit_index + bpb - 1) / 64;
          let shift = bit_index % 64;
          let id = self.get_palette(Pos::new(x, y, z));
          if first == second {
            // Clear the bits of the new id
            new_data[first] &= !(((1 << bpb) - 1) << shift);
            // Set the new id
            new_data[first] |= (id as u64) << shift;
          } else {
            let second_shift = 64 - shift;
            // Clear the bits of the new id
            new_data[first] &= !(((1 << bpb) - 1) << shift);
            new_data[second] &= !(((1 << bpb) - 1) >> second_shift);
            // Set the new id
            new_data[first] |= (id as u64) << shift;
            new_data[second] |= (id as u64) >> second_shift;
          }
          bit_index += bpb;
        }
      }
    }
    self.data = new_data;
    self.bits_per_block += 1;
  }
}

impl ChunkSection for Section {
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    let prev = self.get_palette(pos);
    let palette_id = if let Some(&palette_id) = self.reverse_palette.get(&ty) {
      if prev == palette_id {
        return Ok(());
      }
      self.set_palette(pos, palette_id);
      palette_id
    } else {
      let palette_id = self.insert(ty);
      // Sanity check
      if prev == palette_id {
        unreachable!(
          "while setting {} to {}, prev and palette id were the same ({}, should never happen)",
          pos, ty, prev
        );
      }
      self.set_palette(pos, palette_id);
      palette_id
    };
    self.block_amounts[palette_id as usize] += 1;
    self.block_amounts[prev as usize] -= 1;
    if self.block_amounts[prev as usize] == 0 {
      self.remove(prev);
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
      block_amounts:   self.block_amounts.clone(),
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
    assert_eq!(s.index(Pos::new(15, 15, 15)), (255, 255, 60));

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
  #[test]
  fn test_get_palette() {
    let mut data = vec![0; 16 * 16 * 16 * 4 / 64];
    data[0] = 0xfaf;
    let s = Section { data, ..Default::default() };
    // Sanity check
    assert_eq!(s.get_palette(Pos::new(0, 0, 0)), 0xf);
    assert_eq!(s.get_palette(Pos::new(1, 0, 0)), 0xa);
    assert_eq!(s.get_palette(Pos::new(2, 0, 0)), 0xf);
    assert_eq!(s.get_palette(Pos::new(3, 0, 0)), 0x0);

    let mut data = vec![0; 16 * 16 * 16 * 4 / 64];
    data[0] = 0x1f << 60 | 0x1f << 10 | 0x1f;
    data[1] = 0x1f >> 4;
    let s = Section { bits_per_block: 5, data, ..Default::default() };
    // Make sure it works with split values
    assert_eq!(s.get_palette(Pos::new(0, 0, 0)), 0x1f);
    assert_eq!(s.get_palette(Pos::new(1, 0, 0)), 0x0);
    assert_eq!(s.get_palette(Pos::new(2, 0, 0)), 0x1f);
    assert_eq!(s.get_palette(Pos::new(12, 0, 0)), 0x1f);
  }
  #[test]
  fn test_increase_bits_per_block() {
    let mut s = Section::default();
    // Place some blocks
    s.set_palette(Pos::new(0, 0, 0), 0xf);
    s.set_palette(Pos::new(1, 0, 0), 0x0);
    s.set_palette(Pos::new(2, 0, 0), 0xf);
    s.set_palette(Pos::new(3, 0, 0), 0xa);
    // We want a split value
    s.set_palette(Pos::new(25, 0, 0), 0xf);

    s.increase_bits_per_block();
    // Sanity check
    assert_eq!(s.bits_per_block, 5);
    // Get blocks should work
    assert_eq!(s.get_palette(Pos::new(0, 0, 0)), 0xf);
    assert_eq!(s.get_palette(Pos::new(1, 0, 0)), 0x0);
    assert_eq!(s.get_palette(Pos::new(2, 0, 0)), 0xf);
    assert_eq!(s.get_palette(Pos::new(3, 0, 0)), 0xa);
    assert_eq!(s.get_palette(Pos::new(25, 0, 0)), 0xf);
    // Make sure the data is correct
    assert_eq!(s.data[0], 0xa << 15 | 0xf << 10 | 0xf);
    assert_eq!(s.data[1], 0xf << 61);
    assert_eq!(s.data[2], 0xf >> 3);
  }
  #[test]
  fn test_insert() {
    // Tests the append part
    let mut s = Section::default();
    assert_eq!(s.insert(5), 1);
    assert_eq!(s.palette, vec![0, 5]);
    assert_eq!(s.block_amounts, vec![4096, 0]);
    assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1)].into_iter().collect());
    assert_eq!(s.insert(10), 2);
    assert_eq!(s.palette, vec![0, 5, 10]);
    assert_eq!(s.block_amounts, vec![4096, 0, 0]);
    assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1), (10, 2)].into_iter().collect());

    // Tests the insert part
    let mut s = Section::default();
    assert_eq!(s.insert(10), 1);
    assert_eq!(s.palette, vec![0, 10]);
    assert_eq!(s.block_amounts, vec![4096, 0]);
    assert_eq!(s.reverse_palette, vec![(0, 0), (10, 1)].into_iter().collect());
    assert_eq!(s.insert(5), 1);
    assert_eq!(s.palette, vec![0, 5, 10]);
    assert_eq!(s.block_amounts, vec![4096, 0, 0]);
    assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1), (10, 2)].into_iter().collect());
  }
  #[test]
  fn test_remove() {
    // Tests the pop part
    let mut s = Section {
      palette: vec![0, 5, 10],
      block_amounts: vec![4096, 0, 0],
      reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
      ..Default::default()
    };
    s.remove(2);
    assert_eq!(s.palette, vec![0, 5]);
    assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1)].into_iter().collect());
    s.remove(1);
    assert_eq!(s.palette, vec![0]);
    assert_eq!(s.reverse_palette, vec![(0, 0)].into_iter().collect());

    // Tests the remove part (should affect the elements in the map)
    let mut s = Section {
      palette: vec![0, 5, 10],
      block_amounts: vec![4096, 0, 0],
      reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
      ..Default::default()
    };
    s.remove(1);
    assert_eq!(s.palette, vec![0, 10]);
    assert_eq!(s.reverse_palette, vec![(0, 0), (10, 1)].into_iter().collect());
    s.remove(1);
    assert_eq!(s.palette, vec![0]);
    assert_eq!(s.reverse_palette, vec![(0, 0)].into_iter().collect());
  }
  #[test]
  fn test_set_block() -> Result<(), PosError> {
    // This tests the entire functionality of set_block, assuming that all above
    // tests passed.

    // Sanity check for palette and block amounts
    let mut s = Section::default();
    s.set_block(Pos::new(0, 0, 0), 5)?;
    assert_eq!(s.block_amounts, vec![4095, 1]);
    assert_eq!(s.palette, vec![0, 5]);

    s.set_block(Pos::new(1, 0, 0), 5)?;
    assert_eq!(s.block_amounts, vec![4094, 2]);
    assert_eq!(s.palette, vec![0, 5]);

    s.set_block(Pos::new(1, 0, 0), 0)?;
    assert_eq!(s.block_amounts, vec![4095, 1]);
    assert_eq!(s.palette, vec![0, 5]);

    s.set_block(Pos::new(0, 0, 0), 0)?;
    assert_eq!(s.block_amounts, vec![4096]);
    assert_eq!(s.palette, vec![0]);

    // Make sure that higher palette ids get shifted down correctly.
    let mut s = Section::default();
    s.set_block(Pos::new(0, 0, 0), 10)?;
    assert_eq!(s.block_amounts, vec![4095, 1]);
    assert_eq!(s.palette, vec![0, 10]);

    // 5 should be inserted in the middle
    s.set_block(Pos::new(1, 0, 0), 5)?;
    assert_eq!(s.block_amounts, vec![4094, 1, 1]);
    assert_eq!(s.palette, vec![0, 5, 10]);

    // 10 should be shifted down
    s.set_block(Pos::new(1, 0, 0), 0)?;
    assert_eq!(s.block_amounts, vec![4095, 1]);
    assert_eq!(s.palette, vec![0, 10]);

    // Default state
    s.set_block(Pos::new(0, 0, 0), 0)?;
    assert_eq!(s.block_amounts, vec![4096]);
    assert_eq!(s.palette, vec![0]);

    Ok(())
  }
}

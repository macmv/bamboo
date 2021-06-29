use super::section::Section as ChunkSection;

use crate::{
  math::{Pos, PosError},
  proto,
};
use std::{collections::HashMap, convert::TryFrom};

#[derive(Debug)]
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
  pub(super) fn from_latest_proto(pb: proto::chunk::Section) -> Box<Self> {
    if pb.bits_per_block < 4 || pb.bits_per_block > 64 {
      panic!("invalid bits per block recieved from proto: {}", pb.bits_per_block);
    }
    if pb.palette.len() > 256 {
      panic!("got a palette that was too long: {} > 256", pb.palette.len());
    }
    if let Some(&v) = pb.palette.get(0) {
      if v != 0 {
        panic!("the first element of the palette must be 0, got {}", v);
      }
    }
    if pb.data.len() != 16 * 16 * 16 * pb.bits_per_block as usize / 64 {
      panic!(
        "protobuf data length is incorrect. got {} longs, expected {} longs",
        pb.data.len(),
        16 * 16 * 16 * pb.bits_per_block as usize / 64
      );
    }
    let mut chunk = Section {
      bits_per_block:  pb.bits_per_block as u8,
      data:            pb.data,
      block_amounts:   vec![0; pb.palette.len()],
      palette:         pb.palette,
      reverse_palette: HashMap::new(),
    };
    for (i, &v) in chunk.palette.iter().enumerate() {
      chunk.reverse_palette.insert(v, i as u32);
    }
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          let val = chunk.get_palette(Pos::new(x, y, z));
          chunk.block_amounts[val as usize] += 1;
        }
      }
    }
    Box::new(chunk)
  }
  pub(super) fn from_old_proto(pb: proto::chunk::Section, f: &dyn Fn(u32) -> u32) -> Box<Self> {
    Self::new()
  }
  #[inline(always)]
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
    #[cfg(debug_assertions)]
    if id >= 1 << bpb {
      panic!("passed invalid id {} (must be within 0..{})", id, 1 << bpb);
    }
    let mask = (1 << bpb) - 1;
    if first == second {
      // Clear the bits of the new id
      self.data[first] &= !(mask << shift);
      // Set the new id
      self.data[first] |= (id as u64) << shift;
    } else {
      let second_shift = 64 - shift;
      // Clear the bits of the new id
      self.data[first] &= !(mask << shift);
      self.data[second] &= !(mask >> second_shift);
      // Set the new id
      // TODO: All of these shifts will break on 5+ bits per block. Need to fix.
      self.data[first] |= (id as u64) << shift;
      self.data[second] |= (id as u64) >> second_shift;
    }
  }
  /// Returns the palette id at the given position. This only reads from
  /// `self.data`.
  fn get_palette(&self, pos: Pos) -> u32 {
    let (first, second, shift) = self.index(pos);
    let bpb = self.bits_per_block as usize;
    let mask = (1 << bpb) - 1;
    let val = if first == second {
      // Get the id from data
      (self.data[first] >> shift) & mask
    } else {
      let second_shift = 64 - shift;
      // Get the id from the two values
      (self.data[first] >> shift) & mask | (self.data[second] << second_shift) & mask
    };

    #[cfg(not(debug_assertions))]
    let v = val as u32;
    #[cfg(debug_assertions)]
    let v = u32::try_from(val).unwrap();
    v
  }
  /// This adds a new item to the palette. It will shift all block data, and
  /// extend bits per block (if needed). It will also update the palettes, and
  /// shift the block amounts around. It will not modify the actual amounts in
  /// block_amounts, only the position of each amount. It will insert a 0 into
  /// block_amounts at the index returned. `ty` must not already be in the
  /// palette. Returns the new palette id.
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
    self.shift_all_above(palette_id, 1);
    palette_id
  }
  /// This removes the given palette id from the palette. This includes
  /// modifying the block_amounts array. It will not affect any of the values in
  /// block_amounts, but it will shift the values over if needed. It will also
  /// decrease the bits per block if needed. `id` must be a valid index into
  /// the palette.
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
    self.shift_all_above(id, -1);
  }
  /// This shifts all values in self.data by the given shift value. To clarify,
  /// this just adds shift_amount. It does not bitshift. Used after the palette
  /// has been modified. This also checks if each block id is `>= id`, not `>
  /// id`.
  fn shift_all_above(&mut self, id: u32, shift_amount: i32) {
    let bpb = self.bits_per_block as usize;
    let mut bit_index = 0;
    let mask = (1 << bpb) - 1;
    for _ in 0..16 {
      for _ in 0..16 {
        for _ in 0..16 {
          // Manual implementation of get_palette and set_palette, as calling index()
          // would be slower.
          let first = bit_index / 64;
          let second = (bit_index + bpb - 1) / 64;
          let shift = bit_index % 64;
          let mut val = if first == second {
            // Get the id from data
            self.data[first] >> shift & mask
          } else {
            let second_shift = 64 - shift;
            // Get the id from the two values
            (self.data[first] >> shift) & mask | (self.data[second] << second_shift) & mask
          } as i32;
          if val as u32 >= id {
            val += shift_amount;
            let val = val as u64;
            if first == second {
              // Clear the bits of the new id
              self.data[first] &= !(mask << shift);
              // Set the new id
              self.data[first] |= val << shift;
            } else {
              let second_shift = 64 - shift;
              // Clear the bits of the new id
              self.data[first] &= !(mask << shift);
              self.data[second] &= !(mask >> second_shift);
              // Set the new id
              self.data[first] |= val << shift;
              self.data[second] |= val >> second_shift;
            }
          }
          bit_index += bpb;
        }
      }
    }
  }
  /// Increases the bits per block by one. This will increase
  /// self.bits_per_block, and update the long array. It does not affect the
  /// palette at all.
  fn increase_bits_per_block(&mut self) {
    let bpb = (self.bits_per_block + 1) as usize;
    let mut new_data = vec![0; 16 * 16 * 16 * bpb / 64];
    let mut bit_index = 0;
    let mask = (1 << bpb) - 1;
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          let first = bit_index / 64;
          let second = (bit_index + bpb - 1) / 64;
          let shift = bit_index % 64;
          let id = self.get_palette(Pos::new(x, y, z));
          if first == second {
            // Clear the bits of the new id
            new_data[first] &= !(mask << shift);
            // Set the new id
            new_data[first] |= (id as u64) << shift;
          } else {
            let second_shift = 64 - shift;
            // Clear the bits of the new id
            new_data[first] &= !(mask << shift);
            new_data[second] &= !(mask >> second_shift);
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
    // Currently, this function is almost as fast as it could be. The one limiting
    // factor is with replacing a single unique block with another unique block. If
    // that moves the block data over the bits_per_block threshhold, all of the data
    // will be copied twice. It will also insert and remove from the palette at the
    // same index.
    //
    // This is less than optimal, to say the least. However, the only way this could
    // happen ingame is with a /setblock. This will not happen at all with
    // breaking/placing blocks, as air will always be in the palette. So in
    // survival, this will never come up.
    let mut prev = self.get_palette(pos);
    let palette_id = match self.reverse_palette.get(&ty) {
      Some(&palette_id) => {
        if prev == palette_id {
          // The same block is being placed, so we do nothing.
          return Ok(());
        }
        self.set_palette(pos, palette_id);
        palette_id
      }
      None => {
        let palette_id = self.insert(ty);
        // If insert() was called, and it inserted before prev, the block_amounts would
        // have been shifted, and prev needs to be shifted as well.
        if palette_id <= prev {
          prev += 1;
        }
        self.set_palette(pos, palette_id);
        palette_id
      }
    };
    self.block_amounts[palette_id as usize] += 1;
    self.block_amounts[prev as usize] -= 1;
    if self.block_amounts[prev as usize] == 0 && prev != 0 {
      self.remove(prev);
    }
    Ok(())
  }
  fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError> {
    if min == Pos::new(0, 0, 0) && max == Pos::new(15, 15, 15) {
      // Simple case. We get to just replace the whole section.
      if ty == 0 {
        // With air, this is even easier.
        *self = Section::default();
      } else {
        // With anything else, we need to make sure air stays in the palette.
        *self = Section {
          data: vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64],
          palette: vec![0, ty],
          reverse_palette: vec![(0, 0), (ty, 1)].iter().cloned().collect(),
          block_amounts: vec![0, 4096],
          ..Default::default()
        };
      }
    } else {
      // More difficult case. Here, we need to modify all the block amounts,
      // then remove all the items we need to from the palette. Then, we add the
      // new item to the palette, and update the block data.
      for y in min.y()..=max.y() {
        for z in min.z()..=max.z() {
          for x in min.x()..=max.x() {
            let id = self.get_palette(Pos::new(x, y, z));
            let amt = self.block_amounts[id as usize];
            // Debug assertions mean that we cannot subtract with overflow here.
            self.block_amounts[id as usize] = amt - 1;
          }
        }
      }
      let mut ids_to_remove = vec![];
      for (id, amt) in self.block_amounts.iter().enumerate() {
        #[cfg(debug_assertions)]
        if *amt > 4096 {
          dbg!(&self);
          unreachable!("amount is invalid! should not be possible")
        }
        // Make sure we do not remove air from the palette.
        if *amt == 0 && id != 0 {
          ids_to_remove.push(id as u32);
        }
      }
      for id in ids_to_remove {
        self.remove(id);
      }
      let palette_id = match self.reverse_palette.get(&ty) {
        Some(&palette_id) => palette_id,
        None => self.insert(ty),
      };
      self.block_amounts[palette_id as usize] +=
        ((max.x() - min.x() + 1) * (max.y() - min.y() + 1) * (max.z() - min.z() + 1)) as u32;
      for y in min.y()..=max.y() {
        for z in min.z()..=max.z() {
          for x in min.x()..=max.x() {
            self.set_palette(Pos::new(x, y, z), palette_id);
          }
        }
      }
    }
    Ok(())
  }
  fn get_block(&self, pos: Pos) -> Result<u32, PosError> {
    let id = self.get_palette(pos);
    Ok(self.palette[id as usize])
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
    proto::chunk::Section {
      palette:        self.palette.clone(),
      bits_per_block: self.bits_per_block.into(),
      non_air_blocks: (4096 - self.block_amounts[0]) as i32,
      data:           self.data.clone(),
    }
  }
  fn to_old_proto(&self, f: &dyn Fn(u32) -> u32) -> proto::chunk::Section {
    proto::chunk::Section {
      palette:        self.palette.iter().map(|v| f(*v)).collect(),
      bits_per_block: self.bits_per_block.into(),
      non_air_blocks: (4096 - self.block_amounts[0]) as i32,
      data:           self.data.clone(),
    }
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
  fn test_shift_data_up() -> Result<(), PosError> {
    // Tests shifting all of the block data (this should happen during insert())

    // This section has two blocks placed, one with id 5, and the other with id 10.
    let mut data = vec![0; 4096];
    data[0] = 0x1002;
    let mut s = Section {
      palette: vec![0, 5, 10],
      block_amounts: vec![4096, 0, 0],
      reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
      data,
      ..Default::default()
    };
    // Should shift the block data up.
    s.insert(3);
    assert_eq!(s.data[0], 0x2003);
    // Should shift some of the block data up.
    s.insert(7);
    assert_eq!(s.data[0], 0x2004);
    Ok(())
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
  fn test_shift_data_down() -> Result<(), PosError> {
    // Tests shifting all of the block data (this should happen during insert())

    // This section has one blocks placed with id 10. This is the situation where
    // data has just been modified to no longer contain 5, and we want to remove it
    // from the palette now.
    let mut data = vec![0; 4096];
    data[0] = 0x2;
    let mut s = Section {
      palette: vec![0, 5, 10],
      block_amounts: vec![4096, 0, 0],
      reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
      data,
      ..Default::default()
    };
    // Should shift the block data down.
    s.remove(1);
    assert_eq!(s.data[0], 0x1);
    // Removing 1 again undefined behavior, as 1 is in the block data now. remove()
    // should never be called with the given palette id present.
    Ok(())
  }
  #[test]
  fn test_set_get_block() -> Result<(), PosError> {
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

    // Make sure that replacing blocks works
    let mut s = Section::default();
    s.set_block(Pos::new(1, 0, 0), 5)?;
    assert_eq!(s.palette, vec![0, 5]);
    assert_eq!(s.block_amounts, vec![4095, 1]);
    s.set_block(Pos::new(1, 0, 0), 10)?;
    assert_eq!(s.palette, vec![0, 10]);
    assert_eq!(s.block_amounts, vec![4095, 1]);

    // Test get block
    let mut s = Section::default();
    s.set_block(Pos::new(0, 0, 0), 10)?;
    assert_eq!(s.get_block(Pos::new(0, 0, 0))?, 10);
    s.set_block(Pos::new(0, 0, 0), 123)?;
    dbg!(&s.reverse_palette);
    assert_eq!(s.get_block(Pos::new(0, 0, 0))?, 123);
    s.set_block(Pos::new(1, 3, 2), 5)?;
    assert_eq!(s.get_block(Pos::new(1, 3, 2))?, 5);
    s.set_block(Pos::new(15, 15, 15), 420)?;
    assert_eq!(s.get_block(Pos::new(15, 15, 15))?, 420);

    Ok(())
  }
  #[test]
  fn test_set_all() -> Result<(), PosError> {
    let mut s = Section::default();
    for x in 0..16 {
      for y in 0..16 {
        for z in 0..16 {
          s.set_block(Pos::new(x, y, z), 20)?;
        }
      }
    }
    assert_eq!(s.data, vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
    assert_eq!(s.palette, vec![0, 20]);
    assert_eq!(s.block_amounts, vec![0, 4096]);

    s.set_block(Pos::new(0, 0, 0), 5)?;

    let mut data = vec![0x2222222222222222; 16 * 16 * 16 * 4 / 64];
    data[0] = 0x2222222222222221;
    assert_eq!(s.data, data);
    assert_eq!(s.palette, vec![0, 5, 20]);
    assert_eq!(s.block_amounts, vec![0, 1, 4095]);

    Ok(())
  }
  #[test]
  fn test_fill() -> Result<(), PosError> {
    let mut s = Section::default();
    s.fill(Pos::new(0, 0, 0), Pos::new(15, 15, 15), 20)?;
    assert_eq!(s.data, vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
    assert_eq!(s.palette, vec![0, 20]);
    assert_eq!(s.block_amounts, vec![0, 4096]);

    s.set_block(Pos::new(0, 0, 0), 5)?;

    let mut data = vec![0x2222222222222222; 16 * 16 * 16 * 4 / 64];
    data[0] = 0x2222222222222221;
    assert_eq!(s.data, data);
    assert_eq!(s.palette, vec![0, 5, 20]);
    assert_eq!(s.block_amounts, vec![0, 1, 4095]);

    let mut s = Section::default();
    s.fill(Pos::new(3, 4, 5), Pos::new(8, 9, 10), 20)?;

    dbg!(&s);
    for x in 0..16 {
      for y in 0..16 {
        for z in 0..16 {
          let expected =
            if x >= 3 && x <= 8 && y >= 4 && y <= 9 && z >= 5 && z <= 10 { 20 } else { 0 };
          assert_eq!(s.get_block(Pos::new(x, y, z))?, expected);
        }
      }
    }
    assert_eq!(s.block_amounts[0] + s.block_amounts[1], 4096);

    Ok(())
  }

  #[test]
  fn test_from_proto() {
    let mut pb = proto::chunk::Section::default();
    pb.bits_per_block = 4;
    pb.data = vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64];
    pb.palette.push(0);
    pb.palette.push(5);

    let s = Section::from_latest_proto(pb);
    assert_eq!(s.data, vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
    assert_eq!(s.palette, vec![0, 5]);
    assert_eq!(s.block_amounts, vec![0, 4096]);
  }
}

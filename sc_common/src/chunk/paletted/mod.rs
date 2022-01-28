use super::section::Section as ChunkSection;

use crate::math::{Pos, PosError, WyHashBuilder};
use sc_transfer::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};
use std::collections::HashMap;

mod bits;
#[cfg(test)]
mod tests;

use bits::BitArray;

#[derive(Debug, Clone)]
pub struct Section {
  data:            BitArray,
  // Each index into palette is a palette id. The values are global ids.
  palette:         Vec<u32>,
  // Each index is a palette id, and the sum of this array must always be 4096 (16 * 16 * 16).
  // When we aren't in paletted mode (palette is empty) then this just contains 1 element, which
  // is the amount of air in this section.
  block_amounts:   Vec<u32>,
  // This maps global ids to palette ids.
  reverse_palette: HashMap<u32, u32, WyHashBuilder>,
  // When switching to direct palette, this is the bpe that will be used.
  max_bpe:         u8,
}

impl MessageWrite for Section {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write(&self.data)?;
    m.write(&self.palette)?;
    m.write(&self.block_amounts)?;
    m.write(&self.max_bpe)?;
    m.write_u32(self.reverse_palette.len() as u32)?;
    for (k, v) in &self.reverse_palette {
      m.write_u32(*k)?;
      m.write_u32(*v)?;
    }
    Ok(())
  }
}
impl MessageRead for Section {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    let data = m.read()?;
    let palette = m.read()?;
    let block_amounts = m.read()?;
    let max_bpe = m.read()?;
    let len = m.read_u32()?;
    let mut reverse_palette = HashMap::with_capacity_and_hasher(len as usize, WyHashBuilder);
    for _ in 0..len {
      reverse_palette.insert(m.read_u32()?, m.read_u32()?);
    }
    Ok(Section { data, palette, block_amounts, reverse_palette, max_bpe })
  }
}

impl Section {
  /// Returns the internal data of this section.
  pub fn data(&self) -> &BitArray { &self.data }
  #[inline(always)]
  fn index(&self, pos: Pos) -> usize { (pos.y() << 8 | pos.z() << 4 | pos.x()) as usize }
  /// Writes a single palette id into self.data.
  #[inline(always)]
  unsafe fn set_palette(&mut self, pos: Pos, id: u32) { self.data.set(self.index(pos), id); }
  /// Returns the palette id at the given position. This only reads from
  /// `self.data`.
  #[inline(always)]
  unsafe fn get_palette(&self, pos: Pos) -> u32 { self.data.get(self.index(pos)) }
  /// This adds a new item to the palette. It will shift all block data, and
  /// extend bits per block (if needed). It will also update the palettes, and
  /// shift the block amounts around. It will not modify the actual amounts in
  /// block_amounts, only the position of each amount. It will insert a 0 into
  /// block_amounts at the index returned. `ty` must not already be in the
  /// palette. Returns the new palette id.
  fn insert(&mut self, ty: u32) -> u32 {
    if self.palette.is_empty() {
      panic!("cannot insert into palette with direct block ids");
    }
    if self.palette.len() + 1 >= 1 << self.data.bpe() as usize {
      if self.data.bpe() >= 8 {
        info!(
          "palette has {} entries, increasing from {} to {} bpe",
          self.palette.len(),
          self.data.bpe(),
          self.max_bpe
        );
        assert!(self.max_bpe - 8 < 32, "max_bpe is too large: {}", self.max_bpe);
        unsafe {
          // SAFETY: We just validated that the new `bpe` will be less than 32, so this is
          // safe.
          self.data.increase_bpe(self.max_bpe - 8);
        }
        // This removes the palette, as we are now using direct values.
        for i in 0..4096 {
          unsafe {
            // SAFETY i is within 0..4096
            let v = self.data.get(i);
            let block_id = self.palette[v as usize];
            self.data.set(i, block_id);
          }
        }
        self.palette.clear();
        self.reverse_palette.clear();
        self.block_amounts.drain(1..);
        return 0;
      } else {
        unsafe {
          // SAFETY: We just made sure that `bpe` is less than 8, so this will never
          // overflow the max `bpe`.
          self.data.increase_bpe(1);
        }
      }
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
    // We want to move everything, including palette_id, up by one
    self.data.shift_all_above(palette_id - 1, 1);
    palette_id
  }
  /// This removes the given palette id from the palette. This includes
  /// modifying the block_amounts array. It will not affect any of the values in
  /// block_amounts, but it will shift the values over if needed. It will also
  /// decrease the bits per block if needed. `id` must be a valid index into
  /// the palette.
  fn remove(&mut self, id: u32) {
    let ty = self.palette[id as usize];
    self.palette.remove(id as usize);
    self.block_amounts.remove(id as usize);
    for (_, p) in self.reverse_palette.iter_mut() {
      if *p > id {
        *p -= 1;
      }
    }
    self.reverse_palette.remove(&ty);
    self.data.shift_all_above(id, -1);
  }
  /// Returns the palette of this chunk.
  pub fn palette(&self) -> &[u32] { &self.palette }
  // Returns the number of non air blocks in this chunk. Because paletted chunks
  // track all the amounts of blocks within the chunk, this is a single Vec
  // lookup.
  pub fn non_air_blocks(&self) -> u32 { 4096 - self.block_amounts[0] }
}

impl ChunkSection for Section {
  fn new(max_bpe: u8) -> Self {
    let mut reverse_palette = HashMap::with_hasher(WyHashBuilder);
    reverse_palette.insert(0, 0);
    Section {
      data: BitArray::new(4),
      palette: vec![0],
      block_amounts: vec![4096],
      reverse_palette,
      max_bpe,
    }
  }
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    // SAFETY: We have validated position, so any get_palette or set_palette calls
    // are now safe.
    let mut prev = unsafe { self.get_palette(pos) };
    let palette_id = match self.reverse_palette.get(&ty) {
      Some(&palette_id) => {
        if prev == palette_id {
          // The same block is being placed, so we do nothing.
          return Ok(());
        }
        unsafe { self.set_palette(pos, palette_id) };
        palette_id
      }
      None => {
        if self.palette.is_empty() {
          unsafe { self.set_palette(pos, ty) };
          if ty == 0 && prev != 0 {
            self.block_amounts[0] += 1;
          }
          if prev == 0 && ty != 0 {
            self.block_amounts[0] -= 1;
          }
          return Ok(());
        } else {
          let palette_id = self.insert(ty);
          if self.palette.is_empty() {
            unsafe { self.set_palette(pos, ty) };
            if ty == 0 && prev != 0 {
              self.block_amounts[0] += 1;
            }
            if prev == 0 && ty != 0 {
              self.block_amounts[0] -= 1;
            }
            return Ok(());
          }
          // If insert() was called, and it inserted before prev, the block_amounts would
          // have been shifted, and prev needs to be shifted as well.
          if palette_id <= prev {
            prev += 1;
          }
          unsafe { self.set_palette(pos, palette_id) };
          palette_id
        }
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
    if min.x() >= 16 || min.x() < 0 || min.y() >= 16 || min.y() < 0 || min.z() >= 16 || min.z() < 0
    {
      return Err(min.err("expected min to be within 0 <= x, y, z < 16".into()));
    }
    if max.x() >= 16 || max.x() < 0 || max.y() >= 16 || max.y() < 0 || max.z() >= 16 || max.z() < 0
    {
      return Err(max.err("expected max to be within 0 <= x, y, z < 16".into()));
    }
    // SAFETY: We have validated position, so any get_palette or set_palette calls
    // are now safe.
    if min == Pos::new(0, 0, 0) && max == Pos::new(15, 15, 15) {
      // Simple case. We get to just replace the whole section.
      if ty == 0 {
        // With air, this is even easier.
        *self = Section::new(self.max_bpe);
      } else {
        // With anything else, we need to make sure air stays in the palette.
        *self = Section {
          data:            BitArray::from_data(4, vec![0x1111111111111111; 4096 * 4 / 64]),
          palette:         vec![0, ty],
          reverse_palette: vec![(0, 0), (ty, 1)].iter().cloned().collect(),
          block_amounts:   vec![0, 4096],
          max_bpe:         self.max_bpe,
        };
      }
    } else {
      // More difficult case. Here, we need to modify all the block amounts,
      // then remove all the items we need to from the palette. Then, we add the
      // new item to the palette, and update the block data.

      // If we are using a direct palette, it is just as fast to just call setblock.
      if self.palette.is_empty() {
        for y in min.y()..=max.y() {
          for z in min.z()..=max.z() {
            for x in min.x()..=max.x() {
              let prev = unsafe { self.get_palette(Pos::new(x, y, z)) };
              if prev == 0 && ty != 0 {
                self.block_amounts[0] -= 1;
              }
              if prev != 0 && ty == 0 {
                self.block_amounts[0] += 1;
              }
              unsafe { self.set_palette(Pos::new(x, y, z), ty) };
            }
          }
        }
        return Ok(());
      }

      for y in min.y()..=max.y() {
        for z in min.z()..=max.z() {
          for x in min.x()..=max.x() {
            let id = unsafe { self.get_palette(Pos::new(x, y, z)) };
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
      // Since removing an id changes the indices of the ids below it, we need to
      // iterate through this in reverse.
      ids_to_remove.reverse();
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
            unsafe { self.set_palette(Pos::new(x, y, z), palette_id) };
          }
        }
      }
    }
    Ok(())
  }
  fn get_block(&self, pos: Pos) -> Result<u32, PosError> {
    if pos.x() >= 16 || pos.x() < 0 || pos.y() >= 16 || pos.y() < 0 || pos.z() >= 16 || pos.z() < 0
    {
      return Err(pos.err("expected a pos within 0 <= x, y, z < 16".into()));
    }
    // SAFETY: We have validated position, so any get_palette or set_palette calls
    // are now safe.
    let id = unsafe { self.get_palette(pos) };
    Ok(if self.palette.is_empty() { id } else { self.palette[id as usize] })
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section {
      data:            self.data.clone(),
      palette:         self.palette.clone(),
      block_amounts:   self.block_amounts.clone(),
      reverse_palette: self.reverse_palette.clone(),
      max_bpe:         self.max_bpe,
    })
  }
}

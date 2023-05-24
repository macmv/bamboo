//! This is kept here as a reference for 1.9-1.15 clients. This is the old
//! chunk data format, which looks like this:
//!
//! ```ignore
//! BPE: 5
//! data:
//! 234 01234 01234 .....
//! ___ _____ ..... 34501
//! ```
//!
//! The point is this type of section allows numbers to wrap between the longs.
//! This is quite simply better than the new format, where the extra bits at the
//! high end of each long are simply left to be zero. I don't know why mojang
//! decided to switch away from this. It may be faster for random lookups, but I
//! have not tested this. I will give mojang the benefit of the doubt on this,
//! and assume they had their reasons.
//!
//! Regardless, a new format is used, which is implemented in `bits.rs`. This
//! format is generated in the proxy, when creating chunk packets, so this file
//! is never needed. It is only a reference for anyone looking for an
//! implementation of the old chunk format.
//!
//! NEVERMIND. I have just spent half an hour reading the Minecraft source code,
//! in order to figure out how to index into the new format. They simply
//! hardcoded 64 multiply, offset, and shift values which (through some integer
//! overflow bullshit) magically work. They just hardcoded the numbers for every
//! possible BPE. This makes the whole thing far more annoying to recreate, and
//! it uses up more memory. Mojang is smoking some ~other~ shit.
//!
//! Update 3: Mojang is still smoking some crazy shit, because their complex
//! nonsense algorithm is completely unneeded. So the new system is simpler and
//! probably faster, but is still implemented terribly in vanilla.

use std::fmt;

/// A resizable element vector. It is always 4096 items long, as that is the
/// safest way to go about things. Any value of bits per block, multiplied by
/// 4096, will always go evenly into 64, which means we never have excess space
/// at the end of the internal vector.
///
/// This is used to separate out some of the nasty bitwise operations, and make
/// the [`Section`](super::Section) code a lot cleaner.
#[derive(Clone, PartialEq)]
pub struct OldBitArray {
  /// Bits per entry
  bpe:             u8,
  /// The actual data
  pub(super) data: Vec<u64>,
}

impl fmt::Debug for OldBitArray {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "BitArray {{")?;
    for v in &self.data {
      self.dbg_binary(f, *v)?;
    }
    writeln!(f, "}}")?;
    Ok(())
  }
}

impl OldBitArray {
  /// Creates a new bit array, with all of the data set to 0. `bpe` is the
  /// number of bits per element in the array. The length of this array will
  /// always be 4096. For normal operation of a chunk, `bpe` should always start
  /// at 4.
  ///
  /// # Panics
  /// - If `bpe` is larger than 31. We could go to 64, but
  ///   [`shift_all_above`](Self::shift_all_above) takes an `i32`, so I have
  ///   chosen to cap `bpe` at 31.
  ///
  /// This is checked all of the time, as invalid bpe will cause a lot of
  /// problems.
  pub fn new(bpe: u8) -> Self {
    assert!(bpe < 32, "bpe of {bpe} is too large (must be less than 32)");
    OldBitArray { bpe, data: vec![0; 4096 * bpe as usize / 64] }
  }

  /// Creates a new bit array from the given data.
  ///
  /// # Panics
  /// - If `bpe` is larger than 31.
  /// - If the data length is not the expected length given the `bpe`. The
  ///   expected length is `4096 * bpe / 64`.
  ///
  /// These are both checked all the time, as this function is typically used to
  /// convert data from protobufs, which can have any data in them.
  pub fn from_data(bpe: u8, data: Vec<u64>) -> Self {
    assert!(bpe < 32, "bpe of {bpe} is too large (must be less than 32)");
    assert_eq!(
      data.len(),
      4096 * bpe as usize / 64,
      "while creating a bit array from existing data, got incorrect len"
    );
    OldBitArray { bpe, data }
  }

  /// This is useful for debugging internal data; it will print out the number
  /// in binary format, with spaces inserted between every element.
  fn dbg_binary(&self, f: &mut fmt::Formatter, val: u64) -> fmt::Result {
    writeln!(
      f,
      "  {}",
      format!("{val:064b}")
        .chars()
        .collect::<Vec<char>>()
        .rchunks(self.bpe.into())
        .map(|arr| arr.iter().collect::<String>())
        .rev()
        .collect::<Vec<String>>()
        .join(" ")
    )
  }

  /// Writes an element into the bit array.
  ///
  /// # Panics
  /// - If `index` is outside of `0..4096`
  /// - If `value` is outside of `0..1 << self.bpe`
  ///
  /// All of these checks are  only performed with debug assertions enabled.
  /// This is because `Section` will never cause these checks to fail if it is
  /// running normally. This is only unsafe with debug assertions disabled.
  ///
  /// # Safety
  /// - Passing in an index outside of `0..4096` will cause this to access
  ///   invalid memory. This is checked with an assert when debug assertions are
  ///   enabled.
  #[inline(always)]
  pub unsafe fn set(&mut self, index: usize, value: u32) {
    #[cfg(debug_assertions)]
    assert!(index < 4096, "index {} is too large (must be less than {})", index, 4096);
    #[cfg(debug_assertions)]
    assert!(
      value < 1 << self.bpe,
      "value {} is too large (must be less than {})",
      value,
      1 << self.bpe
    );
    let bpe: usize = self.bpe.into();
    let lo: usize = (index * bpe) / 64;
    let hi: usize = (index * bpe + bpe - 1) / 64;
    // The bit offset of the smallest bit of lo into the long
    let shift = (index * bpe) as u32 % 64;
    let mask = (1 << bpe) - 1;
    let value = u64::from(value);
    if lo == hi {
      // The value only spans one long
      let l = self.data.get_unchecked_mut(lo);
      *l &= !(mask << shift);
      *l |= value << shift;
    } else {
      // We have a situation where we want to set a number, and we need to split it
      // into two.
      //
      // In this situation, we are working with a Vec<u8>, instead of a Vec<u64>, so
      // I'm going to use 8 where 64 should be.
      //
      // shift = 6 (used to left shift L)
      // 8 - shift = 2 (used to right shift H, and to make the lo mask)
      // bpe - (8 - shift) = 3 (this is used to make the hi mask)
      //
      // Before the move:
      //
      // v -> 0 0 0 H H L L L
      //
      // L = v << shift;       L L L 0 0 0 0 0
      // H = v >> (8 - shift); 0 0 0 0 0 0 H H
      //
      // value ->       H H H | L L
      // long  -> 2 2 2 2 2 2 | 1 1 1 1 1 1
      //
      // So we need to shift L to the left by `shift`, and shift H to the right by
      // `64 - shift`.

      // This mask will match L L 0 0 0
      let lo_mask = (1_u64.wrapping_shl(64 - shift) - 1) << shift;
      // This mask will match 0 0 H H H
      let hi_mask = 1_u64.wrapping_shl(bpe as u32 - (64 - shift)) - 1;

      {
        let l = self.data.get_unchecked_mut(lo);
        *l &= !lo_mask;
        *l |= value.wrapping_shl(shift);
      }
      {
        let h = self.data.get_unchecked_mut(hi);
        *h &= !hi_mask;
        *h |= value.wrapping_shr(64 - shift);
      }
    }
  }
  /// Reads an element from the array. The returned value will always be within
  /// `0..1 << self.bpe`
  ///
  /// # Panics
  /// - If `index` is outside of `0..4096`
  ///
  /// All of these checks are  only performed with debug assertions enabled.
  /// This is because `Section` will never cause these checks to fail if it is
  /// running normally. This is only unsafe with debug assertions disabled.
  ///
  /// # Safety
  /// - Passing in an index outside of `0..4096` will cause this to access
  ///   invalid memory. This is checked with an assert when debug assertions are
  ///   enabled.
  #[inline(always)]
  pub unsafe fn get(&self, index: usize) -> u32 {
    #[cfg(debug_assertions)]
    assert!(index < 4096, "index {} is too large (must be less than {})", index, 4096);
    let bpe: usize = self.bpe.into();
    let lo: usize = (index * bpe) / 64;
    let hi: usize = (index * bpe + bpe - 1) / 64;
    // The bit offset of the smallest bit of lo into the long
    let shift = (index * bpe) as u32 % 64;
    let mask = (1 << bpe) - 1;
    let res = if lo == hi {
      // The value only spans one long
      self.data.get_unchecked(lo).wrapping_shr(shift) & mask
    } else {
      // We have a situation where we want to get a number, but it is split between
      // two longs.
      //
      // In this situation, we are working with a Vec<u8>, instead of a Vec<u64>, so
      // I'm going to use 8 where 64 should be.
      //
      // shift = 6 (used to right shift L)
      // 8 - shift = 2 (used to left shift H, and to make the lo mask)
      // bpe - 8 - shift = 3 (this is used to make the hi mask)
      //
      // L = v << shift;
      // H = v >> (8 - shift);
      //
      // value ->       H H H | L L
      // long  -> 2 2 2 2 2 2 | 1 1 1 1 1 1
      //
      // After the move:
      //
      // lo -> H H 0 0 0
      // hi -> 0 0 L L L
      //
      // So we need to shift L to the right by `shift`, and shift H to the left by
      // `64 - shift`.

      // This mask will match 0 0 L L L
      let lo_mask = 1_u64.wrapping_shl(64 - shift) - 1;
      // This mask will match H H 0 0 0
      let hi_mask = (1_u64.wrapping_shl(bpe as u32 - (64 - shift)) - 1) << (64 - shift);

      let lo = self.data.get_unchecked(lo).wrapping_shr(shift) & lo_mask;
      let hi = self.data.get_unchecked(hi).wrapping_shl(64 - shift) & hi_mask;
      lo | hi
    };
    res as u32
  }
  /// Utility function. This will find all values within the array that are
  /// above `sep`, and add the give `shift_amount` to them. This is commonly
  /// used when inserting an item into the palette; all item in the bit array
  /// that are above that new palette entry must be shifted up by one. Note that
  /// neither of these arguments are indices. Both of these parameters operate
  /// on the values within the array, not where they are.
  ///
  /// Note: `sep` is not inclusive. Only values that are greater than sep will
  /// be changed.
  ///
  /// # Panics
  /// - If `sep` is outside of `0..1 << self.bpe`
  /// - If `shift_amount` is outside of `-(1 << self.bpe) + 1..1 << self.bpe`.
  ///   If `bpe` is 4, then the only valid shift amounts are -15 through 15
  ///   (inclusive).
  /// - If any modified elements go outside of `0..1 << self.bpe`.
  ///
  /// All of these checks are  only performed with debug assertions enabled.
  /// This is because `Section` will never cause these checks to fail if it is
  /// running normally. This is not considered unsafe, because giving invalid
  /// values for this will only cause bad data, but no undefined behavior or
  /// invalid memory access.
  pub fn shift_all_above(&mut self, sep: u32, shift_amount: i32) {
    #[cfg(debug_assertions)]
    assert!(
      sep < 1 << self.bpe,
      "separator {} is too large (must be less than {})",
      sep,
      1 << self.bpe
    );
    #[cfg(debug_assertions)]
    assert!(
      shift_amount > -(1 << self.bpe) && shift_amount < 1 << self.bpe,
      "shift amount {} is outside of bounds {}..{} (exclusive)",
      shift_amount,
      -(1 << self.bpe),
      1 << self.bpe
    );
    for i in 0..4096 {
      // SAFETY: `i` is within 0..4096, so this is safe
      let v = unsafe { self.get(i) as i32 };
      if v > sep as i32 {
        #[cfg(debug_assertions)]
        match v.checked_add(shift_amount) {
          // SAFETY: `i` is within 0..4096, so this is safe
          Some(res) => unsafe { self.set(i, res as u32) },
          None => panic!("while shifting, tried to add {shift_amount} to {v} (got overflow)"),
        }
        #[cfg(not(debug_assertions))]
        // SAFETY: `i` is within 0..4096, so this is safe
        unsafe {
          self.set(i, (v + shift_amount) as u32)
        }
      }
    }
  }
  /// Increases the number of bits per entry by `increase`. This will copy all
  /// of the internal data, and is generally a very slow operation.
  ///
  /// This is only an increase, because making a chunk section smaller is a very
  /// rare situation. If the bpe is going up, then players are most likely
  /// building there. If they remove a bunch of stuff, it will be very likely to
  /// come back. So decreasing `bpe` is almost never worthwhile.
  ///
  /// # Panics
  /// - If `increase` or `self.bpe + increase` is larger than 31. We could go to
  ///   64, but [`shift_all_above`](Self::shift_all_above) takes an `i32`, so I
  ///   have chosen to cap `bpe` at 31.
  ///
  /// All of these checks are  only performed with debug assertions enabled.
  /// This is because `Section` will never cause these checks to fail if it is
  /// running normally. This is only unsafe with debug assertions disabled.
  ///
  /// # Safety
  /// - This will cause undefined behavior if `self.bpe + increase` > 31. With
  ///   debug assertions, this will fail with a panic.
  pub unsafe fn increase_bpe(&mut self, increase: u8) {
    #[cfg(debug_assertions)]
    assert!(increase <= 31 || self.bpe + increase <= 31, "increase is too large");
    let mut new = OldBitArray::new(self.bpe + increase);
    for i in 0..4096 {
      new.set(i, self.get(i));
    }
    self.data = new.data;
    self.bpe = new.bpe;
  }
  /// Clones the internal data. Used for generating protobufs.
  pub fn clone_inner(&self) -> Vec<u64> { self.data.clone() }
  /// Returns the number of bits that every element in this array uses.
  pub fn bpe(&self) -> u8 { self.bpe }
  /// Returns the inner long array.
  pub fn long_array(&self) -> &[u64] { &self.data }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get() {
    let data: Vec<u64> = vec![u64::MAX; 4096 * 5 / 64];
    let arr = OldBitArray { bpe: 5, data };

    for i in 0..4096 {
      unsafe {
        assert_eq!(arr.get(i), 31, "failed at {i}");
      }
    }

    let data: Vec<u64> = vec![u64::MAX; 4096 * 4 / 64];
    let arr = OldBitArray { bpe: 4, data };

    for i in 0..4096 {
      unsafe {
        assert_eq!(arr.get(i), 15, "failed at {i}");
      }
    }

    let data: Vec<u64> = vec![0x7777777777777777; 4096 * 4 / 64];
    let arr = OldBitArray { bpe: 4, data };

    for i in 0..4096 {
      unsafe {
        assert_eq!(arr.get(i), 7, "failed at {i}");
      }
    }
  }

  // This test assumes that get has passed
  #[test]
  fn test_set() {
    for bpe in 2..32 {
      let mut arr = OldBitArray::new(bpe);
      let max = 1 << bpe;

      for i in 0..4096 {
        unsafe {
          arr.set(i, i as u32 % max);
          assert_eq!(arr.get(i), i as u32 % max, "failed at index {i}");
        }
      }
    }
  }

  // Below are old Section tests that fit better here now.

  #[test]
  fn test_set_palette() {
    unsafe {
      let mut a = OldBitArray::new(4);
      // Sanity check
      a.set(0, 0xf);
      assert_eq!(a.data[0], 0xf);
      // Sanity check
      a.set(2, 0xf);
      assert_eq!(a.data[0], 0xf0f);
      // Should work up to the edge of the long
      a.set(15, 0xf);
      assert_eq!(a.data[0], 0xf000000000000f0f);
      // Clearing bits should work
      a.set(15, 0x3);
      assert_eq!(a.data[0], 0x3000000000000f0f);

      let mut a = OldBitArray::new(5);
      // Sanity check
      a.set(0, 0x1f);
      assert_eq!(a.data[0], 0x1f);
      // Sanity check
      a.set(2, 0x1f);
      assert_eq!(a.data[0], 0x1f << 10 | 0x1f);
      // Should split the id correctly
      a.set(12, 0x1f);
      assert_eq!(a.data[0], 0x1f << 60 | 0x1f << 10 | 0x1f);
      assert_eq!(a.data[1], 0x1f >> 4);
      a.set(25, 0x1f);
      assert_eq!(a.data[1], 0x1f << 61 | 0x1f >> 4);
      assert_eq!(a.data[2], 0x1f >> 3);
      // Clearing bits should work
      a.set(0, 0x3);
      assert_eq!(a.data[0], 0x1f << 60 | 0x1f << 10 | 0x03);
    }
  }
  #[test]
  fn test_get_palette() {
    unsafe {
      let mut data = vec![0; 16 * 16 * 16 * 4 / 64];
      data[0] = 0xfaf;
      let a = OldBitArray::from_data(4, data);
      // Sanity check
      assert_eq!(a.get(0), 0xf);
      assert_eq!(a.get(1), 0xa);
      assert_eq!(a.get(2), 0xf);
      assert_eq!(a.get(3), 0x0);

      let mut data = vec![0; 16 * 16 * 16 * 5 / 64];
      data[0] = 0x1f << 60 | 0x1f << 10 | 0x1f;
      data[1] = 0x1f >> 4;
      let a = OldBitArray::from_data(5, data);
      // Make sure it works with split values
      assert_eq!(a.get(0), 0x1f);
      assert_eq!(a.get(1), 0x0);
      assert_eq!(a.get(2), 0x1f);
      assert_eq!(a.get(12), 0x1f);
    }
  }
}

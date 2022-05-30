//! Note that this is the new chunk data format, used in 1.16+. See
//! `bits-old.rs` for the previous implementation, which works on 1.9-1.15
//! clients. This new implementation is converted into that old implementation
//! on the proxy.

use super::OldBitArray;
use std::fmt;

/// A resizable element vector. It is always 4096 items long, as that is the
/// safest way to go about things. Any value of bits per block, multiplied by
/// 4096, will always go evenly into 64, which means we never have excess space
/// at the end of the internal vector.
///
/// This is used to separate out some of the nasty bitwise operations, and make
/// the [`Section`](super::Section) code a lot cleaner.
#[derive(Clone, PartialEq)]
pub struct BitArray {
  /// Bits per entry
  bpe:  u8,
  /// The actual data
  data: Vec<u64>,
}

// This impl uses a byte array for `data`, instead of a varint array. After some
// testing in the debug world, it turns out the varint array is smaller in all
// of the cases that I tested it on. I have not done any benchmarks, so this
// implementation might be faster (in which case we want to use it). I am going
// to leave this commented for now, as I don't think it would be significantly
// faster.
use bb_transfer::{
  MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, StructRead, StructReader,
  WriteError,
};
impl MessageRead<'_> for BitArray {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { m.read_struct() }
}
impl StructRead<'_> for BitArray {
  fn read_struct(mut m: StructReader) -> Result<Self, ReadError> {
    let bpe = m.must_read::<u8>(0)?;
    let bytes = m.must_read::<&[u8]>(1)?;
    let mut data = Vec::with_capacity(bytes.len() / 8);
    for v in bytes.chunks(8) {
      data.push(u64::from_le_bytes(v.try_into().unwrap()));
    }
    Ok(BitArray { bpe, data })
  }
}
impl MessageWrite for BitArray {
  fn write<W: std::io::Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    let mut bytes = Vec::with_capacity(self.data.len() * 8);
    for v in &self.data {
      bytes.extend(v.to_le_bytes());
    }
    m.write_struct(2, |m| {
      m.write::<u8>(&self.bpe)?;
      m.write::<&[u8]>(&bytes.as_slice())
    })
  }
}

impl fmt::Debug for BitArray {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "BitArray {{")?;
    for v in &self.data {
      self.dbg_binary(f, *v)?;
    }
    writeln!(f, "}}")?;
    Ok(())
  }
}

impl BitArray {
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
    assert!(bpe < 32, "bpe of {} is too large (must be less than 32)", bpe);
    let epl = 64 / bpe as usize;
    let len = (4096 + epl - 1) / epl;
    BitArray { bpe, data: vec![0; len] }
  }

  /// Creates a new bit array from the given data.
  ///
  /// # Panics
  /// - If `bpe` is larger than 31.
  /// - If the data length is not the expected length given the `bpe`. The
  ///   expected length is `(4096 + (64 / bpe) - 1) / (64 / bpe)`.
  ///
  /// These are both checked all the time, as this function is typically used to
  /// convert data from protobufs, which can have any data in them.
  pub fn from_data(bpe: u8, data: Vec<u64>) -> Self {
    assert!(bpe < 32, "bpe of {} is too large (must be less than 32)", bpe);
    let epl = 64 / bpe as usize;
    let len = (4096 + epl - 1) / epl;
    assert_eq!(data.len(), len, "while creating a bit array from existing data, got incorrect len");
    BitArray { bpe, data }
  }

  /// This is useful for debugging internal data; it will print out the number
  /// in binary format, with spaces inserted between every element.
  fn dbg_binary(&self, f: &mut fmt::Formatter, val: u64) -> fmt::Result {
    writeln!(
      f,
      "  {}",
      format!("{:064b}", val)
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
    let epl = 64 / self.bpe as usize;
    let bpe: usize = self.bpe.into();
    let idx = index / epl;
    let shift = (index % epl) * bpe;
    let mask = (1 << self.bpe as u64) - 1;
    let value = u64::from(value);
    let l = self.data.get_unchecked_mut(idx);
    *l &= !(mask << shift);
    *l |= value << shift;
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
    let epl = 64 / self.bpe as usize;
    let bpe: usize = self.bpe.into();
    let idx = index / epl;
    let shift = (index % epl) * bpe;
    let mask = (1 << self.bpe as u64) - 1;
    ((self.data.get_unchecked(idx) >> shift) & mask) as u32
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
          None => panic!("while shifting, tried to add {} to {} (got overflow)", shift_amount, v),
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
    let mut new = BitArray::new(self.bpe + increase);
    for i in 0..4096 {
      new.set(i, self.get(i));
    }
    self.data = new.data;
    self.bpe = new.bpe;
  }
  /// Clones the internal data. Used for generating protobufs.
  pub fn clone_inner(&self) -> Vec<u64> { self.data.clone() }
  /// Returns the internal data, without cloning.
  pub fn into_inner(self) -> Vec<u64> { self.data }
  /// Returns the number of bits that every element in this array uses.
  pub fn bpe(&self) -> u8 { self.bpe }
  /// Returns the inner long array.
  pub fn long_array(&self) -> &[u64] { &self.data }

  /// Returns the inner long array, using the old format (used in 1.9-1.15).
  /// This needs to copy all the data over to this older format, so this is not
  /// a cheap operation.
  pub fn old_long_array(&self) -> Vec<u64> {
    if self.bpe == 4 {
      return self.data.clone();
    }
    let mut data = OldBitArray::new(self.bpe);
    for i in 0..4096 {
      // SAFETY: `i` is within 0..4096
      unsafe {
        data.set(i, self.get(i));
      }
    }
    data.data
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get() {
    let data: Vec<u64> = vec![u64::MAX; (4096 + (64 / 5) - 1) / (64 / 5)];
    let arr = BitArray { bpe: 5, data };

    for i in 0..4096 {
      unsafe {
        assert_eq!(arr.get(i), 31, "failed at {}", i);
      }
    }

    let data: Vec<u64> = vec![u64::MAX; (4096 + (64 / 4) - 1) / (64 / 4)];
    let arr = BitArray { bpe: 4, data };

    for i in 0..4096 {
      unsafe {
        assert_eq!(arr.get(i), 15, "failed at {}", i);
      }
    }

    let data: Vec<u64> = vec![0x7777777777777777; (4096 + (64 / 4) - 1) / (64 / 4)];
    let arr = BitArray { bpe: 4, data };

    for i in 0..4096 {
      unsafe {
        assert_eq!(arr.get(i), 7, "failed at {}", i);
      }
    }
  }

  // This test assumes that get has passed
  #[test]
  fn test_set() {
    for bpe in 2..32 {
      let mut arr = BitArray::new(bpe);
      let max = 1 << bpe;

      for i in 0..4096 {
        unsafe {
          arr.set(i, i as u32 % max);
          assert_eq!(arr.get(i), i as u32 % max, "failed at index {}", i);
        }
      }
    }
  }

  // Below are old Section tests that fit better here now.

  #[test]
  fn test_set_palette() {
    unsafe {
      let mut a = BitArray::new(4);
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

      let mut a = BitArray::new(5);
      // Sanity check
      a.set(0, 0x1f);
      assert_eq!(a.data[0], 0x1f);
      // Sanity check
      a.set(2, 0x1f);
      assert_eq!(a.data[0], 0x1f << 10 | 0x1f);
      // Should not split the id
      a.set(12, 0x1f);
      assert_eq!(a.data[0], 0x1f << 10 | 0x1f);
      assert_eq!(a.data[1], 0x1f);
      a.set(24, 0x1f);
      assert_eq!(a.data[1], 0x1f);
      assert_eq!(a.data[2], 0x1f);
      // Clearing bits should work
      a.set(0, 0x3);
      assert_eq!(a.data[0], 0x1f << 10 | 0x03);
    }
  }
  #[test]
  fn test_get_palette() {
    unsafe {
      let mut data = vec![0; (4096 + (64 / 4) - 1) / (64 / 4)];
      data[0] = 0xfaf;
      let a = BitArray::from_data(4, data);
      // Sanity check
      assert_eq!(a.get(0), 0xf);
      assert_eq!(a.get(1), 0xa);
      assert_eq!(a.get(2), 0xf);
      assert_eq!(a.get(3), 0x0);

      let mut data = vec![0; (4096 + (64 / 5) - 1) / (64 / 5)];
      // The new format doesn't have split longs, so this ends up being very simple.
      data[0] = 0x1f << 10 | 0x1f;
      data[1] = 0x1f;
      let a = BitArray::from_data(5, data);
      // Make sure it works with split values
      assert_eq!(a.get(0), 0x1f);
      assert_eq!(a.get(1), 0x0);
      assert_eq!(a.get(2), 0x1f);
      assert_eq!(a.get(12), 0x1f);
    }
  }
}

/// A resizable element vector. It is always 4096 items long, as that is the
/// safest way to go about things. Any value of bits per block, multiplied by
/// 4096, will always go evenly into 64, which means we never have excess space
/// at the end of the internal vector.
///
/// This is used to separate out some of the nasty bitwise operations, and make
/// the [`Section`](super::Section) code a lot cleaner.
pub struct BitArray {
  /// Bits per entry
  bpe:  u8,
  /// The actual data
  data: Vec<u64>,
}

impl BitArray {
  /// Creates a new bit array, with all of the data set to 0. `bpe` is the
  /// number of bits per element in the array. The length of this array will
  /// always be 4096. For normal operation of a chunk, `bpe` should always start
  /// at 4.
  pub fn new(bpe: u8) -> Self {
    BitArray { bpe, data: vec![0; 16 * 16 * 16 * bpe as usize / 64] }
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
  /// # Panics
  /// - If `sep` is outside of `0..1 << self.bpe`
  /// - If `shift_amount` is outside of `-(1 << self.bpe) + 1..1 << self.bpe`.
  ///   If `bpe` is 4, then the only valid shift amounts are -15 through 15
  ///   (inclusive).
  /// - If any modified elements go outside of `0..1 << self.bpe`.
  ///
  /// All of these checks are  only performed with debug assertions enabled.
  /// This is because `Section` will never cause these checks to fail if it is
  /// running normally. This is only unsafe with debug assertions disabled.
  pub unsafe fn shift_all_above(&mut self, sep: u32, shift_amount: i32) {
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
      // self.get() will always return a positive `i32`.
      let v = self.get(i) as i32;
      if v > sep as i32 {
        self.set(i, (v + shift_amount) as u32);
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
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get() {
    let data: Vec<u64> = vec![u64::MAX; 4096 * 5 / 64];
    let arr = BitArray { bpe: 5, data };

    for i in 0..4096 {
      unsafe {
        assert_eq!(arr.get(i), 31, "expected 31 at {}", i);
      }
    }
  }
}

/// A resizable element vector. It is always 4096 items long, as that is the
/// safest way to go about things. Any value of bits per block, multiplied by
/// 4096, will always go evenly into 64, which means we never have excess space
/// at the end of the internal vector.
///
/// This is used to seperate out some of the nasty bitwise operations, and make
/// the [`Section`] code a lot cleaner.
pub struct BitArray {
  // Bits per entry
  bpe:  u8,
  data: Vec<u64>,
}

impl BitArray {
  /// Creates a new bit array, with all of the data set to 0. BPE is the number
  /// of bits per element in the array. The length of this array will always be
  /// 4096.
  pub fn new(bpe: u8) -> Self {
    BitArray { bpe, data: vec![0; 16 * 16 * 16 * bpe as usize / 64] }
  }

  /// Writes an element into the bit array.
  ///
  /// # Panics
  /// - If `index` is outside of `0..4096`
  /// - If `value` is outside of `0..1 << self.bpe`
  pub fn set(&mut self, index: usize, value: u32) {}
  /// Reads an element from the array. The returned value will always be within
  /// `0..1 << self.bpe`
  ///
  /// # Panics
  /// - If `index` is outside of `0..4096`
  pub fn get(&self, index: usize) -> u32 {
    0
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
  /// - If any modified elments go outside of `0..1 << self.bpe`. This is only
  ///   checked with debug assertions enabled.
  pub fn shift_all_above(&mut self, sep: u32, shit_amount: i32) {}
}

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

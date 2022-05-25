use super::Rng;

pub struct Xoroshiro {
  seed: i64,
}

impl Xoroshiro {
  pub fn new(seed: i64) -> Xoroshiro {
    let mut rng = Xoroshiro { seed: 0 };
    rng.set_seed(seed);
    rng
  }
}

impl Rng for Xoroshiro {
  fn set_seed(&mut self, seed: i64) { self.seed = (seed ^ 0x5DEECE66D) & 0xFFFFFFFFFFFF; }

  fn next_bits(&mut self, bits: i32) -> i32 {
    self.seed = (self.seed.wrapping_mul(25214903917) + 11) & 0xFFFFFFFFFFFF;
    (self.seed >> (48 - bits)) as i32
  }
  fn next_int(&mut self) -> i32 { self.next_bits(32) }
  fn next_int_max(&mut self, max: i32) -> i32 {
    if (max & max - 1) == 0 {
      return (max as i64 * (self.next_bits(31) as i64) >> 31) as i32;
    }
    let mut k;
    loop {
      let j = self.next_bits(31);
      k = j % max;
      if j - k + (max - 1) >= 0 {
        break;
      }
    }
    k
  }

  fn next_between(&mut self, min: i32, max: i32) -> i32 { self.next_int_max(max - min + 1) + min }

  fn next_long(&mut self) -> i64 {
    let i = self.next_bits(32);
    let j = self.next_bits(32);
    (i as i64) << 32 + j as i64
  }

  fn next_boolean(&mut self) -> bool { true }
  fn next_float(&mut self) -> f32 { 0.0 }
  fn next_double(&mut self) -> f64 {
    let i = self.next_bits(26);
    let j = self.next_bits(27);
    let l = ((i as i64) << 27) + j as i64;
    l as f64 * 1.110223E-16
  }
  fn next_gaussian(&mut self) -> f64 { 0.0 }

  fn skip(&mut self, count: usize) {
    for _ in 0..count {
      self.next_int();
    }
  }
}

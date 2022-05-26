use super::{Rng, RngDeriver};

pub struct SimpleRng {
  seed: i64,
}

pub struct SimpleRngDeriver {
  seed: i64,
}

impl SimpleRng {
  pub fn new(seed: i64) -> SimpleRng {
    let mut rng = SimpleRng { seed: 0 };
    rng.set_seed(seed);
    rng
  }

  pub fn derive(&mut self) -> SimpleRng { todo!() }

  fn next_bits(&mut self, bits: i32) -> i32 {
    self.seed = (self.seed.wrapping_mul(25214903917) + 11) & 0xFFFFFFFFFFFF;
    (self.seed >> (48 - bits)) as i32
  }
}

impl SimpleRngDeriver {
  pub(super) fn new(seed: i64) -> Self { SimpleRngDeriver { seed } }
}

impl RngDeriver<SimpleRng> for SimpleRngDeriver {
  fn create_rng(&self, name: &str) -> SimpleRng { SimpleRng::new(java_hash(name).into()) }
}

fn java_hash(text: &str) -> i32 {
  let mut hash = 0;
  let len = text.len() as u32;
  for (i, b) in text.bytes().enumerate() {
    hash += (b as i32 * 31).wrapping_pow(len - (i as u32 + 1));
  }
  hash
}

impl Rng for SimpleRng {
  type Deriver = SimpleRngDeriver;
  fn create_deriver(&mut self) -> SimpleRngDeriver { SimpleRngDeriver::new(self.next_long()) }

  fn set_seed(&mut self, seed: i64) { self.seed = (seed ^ 0x5DEECE66D) & 0xFFFFFFFFFFFF; }

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
    ((i as i64) << 32) + j as i64
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

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn basic_next_int() {
    let mut rng = SimpleRng::new(1);
    assert_eq!(rng.next_int(), -1155869325);
    assert_eq!(rng.next_int(), 431529176);
  }
  #[test]
  fn basic_next_double() {
    let mut rng = SimpleRng::new(1);
    assert_similar(rng.next_double(), 0.730878);
    assert_similar(rng.next_double(), 0.410080);
  }

  #[test]
  fn lots_of_calls() {
    let mut rng = SimpleRng::new(1);
    assert_eq!(rng.next_int(), -1155869325);
    assert_similar(rng.next_double(), 0.100473);
    assert_eq!(rng.next_int(), 1749940626);
  }

  #[test]
  fn next_int_max() {
    let mut rng = SimpleRng::new(1);
    assert_eq!(rng.next_int_max(5), 0);
    assert_eq!(rng.next_int_max(5), 3);
    assert_eq!(rng.next_int_max(5), 2);
    assert_eq!(rng.next_int_max(5), 3);
  }

  fn assert_similar(actual: f64, expected: f64) {
    if (expected - actual).abs() > 0.0001 {
      panic!("Expected: {expected}, got: {actual}");
    }
  }
}

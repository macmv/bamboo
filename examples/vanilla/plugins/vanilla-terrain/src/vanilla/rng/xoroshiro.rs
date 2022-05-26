use super::{Rng, RngDeriver};

struct Impl {
  lo: u64,
  hi: u64,
}

impl Impl {
  pub fn new(lo: u64, hi: u64) -> Self {
    if lo | hi == 0 {
      Impl { lo: 11400714819323198485, hi: 7640891576956012809 }
    } else {
      Impl { lo, hi }
    }
  }
  pub fn next(&mut self) -> u64 {
    let l = self.lo;
    let mut m = self.hi;
    let n = l.wrapping_add(m).rotate_left(17).wrapping_add(l);
    m ^= l;
    self.lo = l.rotate_left(49) ^ m ^ (m << 21);
    self.hi = m.rotate_left(28);
    n
  }
}

pub struct Xoroshiro {
  imp: Impl,
}
pub struct XoroshiroDeriver {
  lo: u64,
  hi: u64,
}

fn split_mix(mut seed: u64) -> u64 {
  seed = (seed ^ seed >> 30).wrapping_mul(13787848793156543929);
  seed = (seed ^ seed >> 27).wrapping_mul(10723151780598845931);
  seed ^ seed >> 31
}

fn xoroshiro_seed(seed: u64) -> (u64, u64) {
  let l = seed ^ 0x6a09e667f3bcc909;
  let m = l.wrapping_sub(7046029254386353131);
  (split_mix(l), split_mix(m))
}

impl Xoroshiro {
  pub fn new(seed: i64) -> Xoroshiro {
    let (lo, hi) = xoroshiro_seed(seed as u64);
    Xoroshiro::new_long(lo, hi)
  }
  pub fn new_long(lo: u64, hi: u64) -> Xoroshiro { Xoroshiro { imp: Impl::new(lo, hi) } }

  fn next_bits(&mut self, bits: i32) -> i64 { (self.imp.next() >> 64 - bits) as i64 }
}

impl RngDeriver for XoroshiroDeriver {
  type Rng = Xoroshiro;

  fn create_rng(&self, name: &str) -> Xoroshiro {
    let bytes = md5::compute(name).0;
    let num = u128::from_le_bytes(bytes);
    let lo = num as u64;
    let hi = (num >> 64) as u64;
    return Xoroshiro::new_long(lo ^ self.lo, hi ^ self.hi);
  }
}

impl Rng for Xoroshiro {
  type Deriver = XoroshiroDeriver;
  fn create_deriver(&mut self) -> XoroshiroDeriver {
    XoroshiroDeriver { lo: self.imp.next(), hi: self.imp.next() }
  }

  fn set_seed(&mut self, seed: i64) {
    let (lo, hi) = xoroshiro_seed(seed as u64);
    self.imp = Impl::new(lo, hi);
  }

  fn next_int(&mut self) -> i32 { self.imp.next() as i32 }
  fn next_int_max(&mut self, max: i32) -> i32 {
    let mut l = self.next_int() as u64 & 0xffffffff;
    let mut m = l * max as u64;
    let mut n = m & 0xffffffff;
    if n < max as u64 {
      let j = (!max + 1) % max;
      while n < j as u64 {
        l = self.next_int() as u64 & 0xffffffff;
        m = l * max as u64;
        n = m & 0xffffffff;
      }
    }
    return (m >> 32) as i32;
  }

  fn next_between(&mut self, min: i32, max: i32) -> i32 { self.next_int_max(max - min + 1) + min }

  fn next_long(&mut self) -> i64 { self.imp.next() as i64 }

  fn next_boolean(&mut self) -> bool { self.imp.next() != 0 }
  fn next_float(&mut self) -> f32 { self.next_bits(24) as f32 * 5.9604645E-8 }
  fn next_double(&mut self) -> f64 { self.next_bits(53) as f64 * 1.110223E-16 }
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

  #[test]
  fn basic_next_int() {
    let mut rng = Xoroshiro::new(0);
    assert_eq!(rng.next_int(), -160476802);
    assert_eq!(rng.next_int(), 781697906);
  }
}

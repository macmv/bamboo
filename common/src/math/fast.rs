use std::f64::consts::PI;

pub trait FastMath {
  /// A faster, and less accurate version of cos. This should use a lookup table
  /// to produce results very quickly.
  fn fast_cos(&self) -> Self
  where
    Self: Sized;

  /// A faster, and less accurate version of sin. This should use a lookup table
  /// to produce results very quickly.
  fn fast_sin(&self) -> Self
  where
    Self: Sized;
}

// Number of elements between 0 and pi/2
const TABLE_SIZE: usize = 512;
const HALF_PI: f64 = PI / 2.0;
const HALF_PI_TO_INDEX: f64 = HALF_PI * (TABLE_SIZE as f64);

const COS_LOOKUP: [f64; TABLE_SIZE] =
  sc_macros::lookup_table!(min: 0.0, max: 1.57079632679, steps: 512, ty: f64, func: cos);

impl FastMath for f64 {
  fn fast_cos(&self) -> f64 {
    if self.is_nan() {
      return *self;
    }
    COS_LOOKUP[((self % HALF_PI).abs() * HALF_PI_TO_INDEX) as usize]
  }
  fn fast_sin(&self) -> f64 {
    self.cos()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn f64_fast_cos() {
    for i in 0..100 {
      let val = i as f64 / 10.0;
      assert_close(val.fast_cos(), val.cos());
    }

    assert!(f64::NAN.fast_cos().is_nan(), "fast_cos of nan is not nan");
  }

  fn assert_close(a: f64, b: f64) {
    assert!(a > b - 0.03, "values differ: {} {}", a, b);
    assert!(a < b + 0.03, "values differ: {} {}", a, b);
  }
}

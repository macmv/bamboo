/// This trait is implemented on f64 and f32. It provides faster and less
/// accurate versions of `cos` and `sin`, and will do other useful things in the
/// future.
pub trait FastMath {
  /// A faster, and less accurate version of `cos`. This uses a lookup table
  /// to produce results very quickly.
  ///
  /// This will return results within +/- [`EPSILON`] of the actual value.
  fn fast_cos(&self) -> Self
  where
    Self: Sized;

  /// A faster, and less accurate version of `sin`. This uses a lookup table
  /// to produce results very quickly.
  ///
  /// This will return results within +/- [`EPSILON`] of the actual value.
  fn fast_sin(&self) -> Self
  where
    Self: Sized;
}
/// This is how close the [`fast_cos`](FastMath::fast_cos) and
/// [`fast_sin`](FastMath::fast_sin) functions will be to the real output. The
/// only way this can be improved is if the lookup table were expanded, but this
/// number is small enough for my purposes.
///
/// This value returned can differ by +/- `EPSILON`. The way this is tested is
/// with a function like so:
/// ```
/// fn assert_close(a: f64, b: f64) {
///   assert!(a > b - EPSILON, "values differ: {} {}", a, b);
///   assert!(a < b + EPSILON, "values differ: {} {}", a, b);
/// }
/// ```
///
/// This value could be decreased down to `0.004`, if the `fast_cos` and
/// `fast_sin` functions were improved. Currently, they do most of the math with
/// longs, which speeds things up a lot. However, this reduces the accuracy by a
/// lot.
pub const EPSILON: f64 = 0.005;
// pub const EPSILON: f64 = 0.004;

// Number of elements between 0 and pi/2
const TABLE_SIZE: usize = 256;

const LOOKUP_F64: [f64; TABLE_SIZE] =
  sc_macros::lookup_table!(min: 0.0, max: 1.57079632679, steps: 256, ty: f64);
const LOOKUP_F32: [f32; TABLE_SIZE] =
  sc_macros::lookup_table!(min: 0.0, max: 1.57079632679, steps: 256, ty: f32);

macro_rules! fast_math_impl {
  ($mod_name:ident, $ty:ident, $lookup:ident) => {
    // Make a seperate module, so that we can have PI constants and such not
    // interfere with f64 vs f32.
    mod $mod_name {
      use super::{$lookup, FastMath, TABLE_SIZE};
      use std::$ty::consts::PI;

      // This is what we are going to multiply the float by. After multiplying the
      // float, we convert it to a long, then do all out logic with that. It improves
      // performance by >50%.
      const EXP: $ty = (1_u64 << 11) as $ty;
      const EXPI: i64 = 1_i64 << 11;
      const PI_2_0: i64 = (PI * 2.0 * EXP) as i64;
      const PI_1_5: i64 = (PI * 1.5 * EXP) as i64;
      const PI_1_0: i64 = (PI * 1.0 * EXP) as i64;
      const PI_0_5: i64 = (PI * 0.5 * EXP) as i64;

      impl FastMath for $ty {
        fn fast_cos(&self) -> $ty {
          if self.is_nan() {
            return *self;
          }
          // Float remainder is slow. The value if a f64 can be stored in 53 bits
          // (ignoring the exponent). So we can multiply this value by 1 ^ 11, and do the
          // rest of the math with integer logic.
          //
          // For large floating point values, this may overflow. However, the accuracy of
          // those floats would already be less than one, so I am going to ignore that
          // case.
          let int = (self * EXP) as i64;
          let m = int.rem_euclid(PI_2_0);
          // We are calculating this: (m / 0.5pi) * TABLE_SIZE
          //
          // We multiple M by EXPI a second time, which is removed by / PI_0_5. The
          // remaining EXPI is removed at the end. This means we will have problems with
          // floats above 1 << 42, but at that point this function is probbaly innacurate
          // anyways.
          let mut idx = ((((m * EXPI) / PI_0_5) * TABLE_SIZE as i64 + (EXPI / 2)) / EXPI) as usize;
          // Quadrants:
          //   ---------
          //  /  2 | 1  \
          // |-----------|
          //  \  3 | 4  /
          //   ---------
          if m <= PI_0_5 {
            // 1st quadrant
            if idx == TABLE_SIZE {
              // If we checked >=, we wouldn't find logic errors in this function. Anything
              // above TABLE_SIZE is invalid.
              0.0
            } else {
              $lookup[idx]
            }
          } else if m <= PI_1_0 {
            // 2nd quadrant
            idx -= TABLE_SIZE;
            if idx == 0 {
              0.0
            } else {
              -$lookup[TABLE_SIZE - idx]
            }
          } else if m <= PI_1_5 {
            // 3rd quadrant
            idx -= TABLE_SIZE * 2;
            if idx == TABLE_SIZE {
              // If we checked >=, we wouldn't find logic errors in this function. Anything
              // above TABLE_SIZE is invalid.
              0.0
            } else {
              -$lookup[idx]
            }
          } else {
            // 4th quadrant
            idx -= TABLE_SIZE * 3;
            if idx == 0 {
              0.0
            } else {
              $lookup[TABLE_SIZE - idx]
            }
          }
        }
        fn fast_sin(&self) -> $ty {
          if self.is_nan() {
            return *self;
          }
          let int = (self * EXP) as i64;
          let m = int.rem_euclid(PI_2_0);
          let mut idx = ((((m * EXPI) / PI_0_5) * TABLE_SIZE as i64 + (EXPI / 2)) / EXPI) as usize;
          // Quadrants:
          //   ---------
          //  /  2 | 1  \
          // |-----------|
          //  \  3 | 4  /
          //   ---------
          if m <= PI_0_5 {
            // 1st quadrant
            if idx == 0 {
              0.0
            } else {
              $lookup[TABLE_SIZE - idx]
            }
          } else if m <= PI_1_0 {
            // 2nd quadrant
            idx -= TABLE_SIZE;
            if idx == TABLE_SIZE {
              0.0
            } else {
              $lookup[idx]
            }
          } else if m <= PI_1_5 {
            // 3rd quadrant
            idx -= TABLE_SIZE * 2;
            if idx == 0 {
              0.0
            } else {
              -$lookup[TABLE_SIZE - idx]
            }
          } else {
            // 4th quadrant
            idx -= TABLE_SIZE * 3;
            if idx == TABLE_SIZE {
              0.0
            } else {
              -$lookup[idx]
            }
          }
        }
      }
    }
  };
}

fast_math_impl!(f64_impl, f64, LOOKUP_F64);
fast_math_impl!(f32_impl, f32, LOOKUP_F32);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn f64_fast_cos() {
    for i in -10000..10000 {
      let val = i as f64 / 1000.0;
      assert_close_f64(val.cos(), val.fast_cos());
    }

    assert!(f64::NAN.fast_cos().is_nan(), "fast_cos of nan is not nan");
  }

  #[test]
  fn f64_fast_sin() {
    for i in -10000..10000 {
      let val = i as f64 / 1000.0;
      assert_close_f64(val.sin(), val.fast_sin());
    }

    assert!(f64::NAN.fast_cos().is_nan(), "fast_cos of nan is not nan");
  }

  #[test]
  fn f32_fast_cos() {
    for i in -10000..10000 {
      let val = i as f32 / 1000.0;
      assert_close_f32(val.cos(), val.fast_cos());
    }

    assert!(f32::NAN.fast_cos().is_nan(), "fast_cos of nan is not nan");
  }

  #[test]
  fn f32_fast_sin() {
    for i in -10000..10000 {
      let val = i as f32 / 1000.0;
      assert_close_f32(val.sin(), val.fast_sin());
    }

    assert!(f32::NAN.fast_cos().is_nan(), "fast_cos of nan is not nan");
  }

  #[track_caller]
  fn assert_close_f64(a: f64, b: f64) {
    println!("real,fast: {:.5} {:.5}", a, b);
    assert!(a > b - EPSILON, "values differ: {} {}", a, b);
    assert!(a < b + EPSILON, "values differ: {} {}", a, b);
  }

  #[track_caller]
  fn assert_close_f32(a: f32, b: f32) {
    println!("real,fast: {:.5} {:.5}", a, b);
    assert!(a > b - EPSILON as f32, "values differ: {} {}", a, b);
    assert!(a < b + EPSILON as f32, "values differ: {} {}", a, b);
  }
}

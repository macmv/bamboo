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
const TABLE_SIZE: usize = 256;

const COS_LOOKUP: [f64; TABLE_SIZE] =
  sc_macros::lookup_table!(min: 0.0, max: 1.57079632679, steps: 256, ty: f64, func: cos);

impl FastMath for f64 {
  fn fast_cos(&self) -> f64 {
    self.cos()
  }
  fn fast_sin(&self) -> f64 {
    self.cos()
  }
}

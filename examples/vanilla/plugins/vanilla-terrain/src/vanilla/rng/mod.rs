mod simple;
mod xoroshiro;

pub use simple::{SimpleRng, SimpleRngDeriver};
pub use xoroshiro::{Xoroshiro, XoroshiroDeriver};

pub trait Rng {
  type Deriver: RngDeriver;

  /// Creates a deriver from this rng.
  fn create_deriver(&mut self) -> Self::Deriver;

  /// Updates the seed of this rng.
  fn set_seed(&mut self, seed: i64);

  fn next_int(&mut self) -> i32;
  fn next_int_max(&mut self, max: i32) -> i32;
  fn next_between(&mut self, min: i32, max: i32) -> i32;
  fn next_long(&mut self) -> i64;
  fn next_boolean(&mut self) -> bool;
  fn next_float(&mut self) -> f32;
  fn next_double(&mut self) -> f64;
  fn next_gaussian(&mut self) -> f64;
  fn skip(&mut self, count: usize);
}

pub trait RngDeriver {
  /// The rng that will be created from this deriver. Note that this isn't
  /// always going to be the same as the rng that created this deriver.
  type Rng: Rng;

  /// Creates an rng from this deriver.
  fn create_rng(&self, name: &str) -> Self::Rng;
}

mod simple;
mod xoroshiro;

pub use simple::{SimpleRng, SimpleRngDeriver};
pub use xoroshiro::{Xoroshiro, XoroshiroDeriver};

pub trait Rng {
  type Deriver: RngDeriver<Self>;

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

pub trait RngDeriver<R: ?Sized> {
  /// Creates an rng from this deriver.
  fn create_rng(&self, name: &str) -> R;
}

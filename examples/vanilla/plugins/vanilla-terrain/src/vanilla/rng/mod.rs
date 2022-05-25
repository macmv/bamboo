mod simple;
mod xoroshiro;

pub use simple::SimpleRng;
pub use xoroshiro::Xoroshiro;

pub trait Rng {
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

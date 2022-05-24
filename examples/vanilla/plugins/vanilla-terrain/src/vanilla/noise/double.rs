use super::{super::rng::Rng, Noise};

pub struct Double<N> {
  first:  N,
  second: N,
}

impl<N> Double<N> {
  pub fn new(first: N, second: N) -> Self { Double { first, second } }
}

impl<N: Noise> Noise for Double<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 { todo!() }
}

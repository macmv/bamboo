use super::{super::rng::Rng, Noise};

pub struct Double<N> {
  first:     N,
  second:    N,
  amplitude: f64,
}

impl<N> Double<N> {
  pub fn new(first: N, second: N, amplitude: f64) -> Self { Double { first, second, amplitude } }
}

impl<N: Noise> Noise for Double<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    let d = x * 1.0181268882175227;
    let e = y * 1.0181268882175227;
    let f = z * 1.0181268882175227;
    (self.first.sample(x, y, z) + self.second.sample(d, e, f)) * self.amplitude
  }
}

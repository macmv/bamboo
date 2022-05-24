use super::{super::rng::Rng, Noise};

pub struct Octave<N> {
  noise:   N,
  octaves: i32,
}

impl<N> Octave<N> {
  pub fn new(noise: N, octaves: i32) -> Self { Octave { noise, octaves } }
}

impl<N: Noise> Noise for Octave<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 { todo!() }
}

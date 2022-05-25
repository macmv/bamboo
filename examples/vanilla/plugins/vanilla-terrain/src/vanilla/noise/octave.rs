use super::{super::rng::Rng, Noise};

pub struct Octave<N> {
  // One for each octave. The tuple contains the noise function and the amplitude of that function.
  samplers:    Vec<(N, f64)>,
  lacunarity:  f64,
  persistence: f64,
}

impl<N> Octave<N> {
  pub fn new<R: Rng>(
    rng: &mut R,
    noise: impl Fn(&mut R) -> N,
    octaves: i32,
    amplitudes: &[f64],
  ) -> Self {
    Octave {
      samplers:    amplitudes.iter().copied().map(|amp| (noise(rng), amp)).collect(),
      lacunarity:  2.0_f64.powi(octaves),
      persistence: 2.0_f64.powi(octaves),
    }
  }
  pub fn get_octave(&self, i: usize) -> &N { &self.samplers[i].0 }
}

pub fn maintain_precision(v: f64) -> f64 { v - (v / 3.3554432E7 + 0.5).floor() * 3.3554432E7 }

impl<N: Noise> Noise for Octave<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    let mut total = 0.0;
    let mut lacunarity = self.lacunarity;
    let mut persistence = self.persistence;
    for (noise, amplitude) in &self.samplers {
      let value = noise.sample(
        maintain_precision(x * lacunarity),
        maintain_precision(y * lacunarity),
        maintain_precision(z * lacunarity),
      );
      total += amplitude * value * persistence;
      lacunarity *= 2.0;
      persistence /= 2.0;
    }
    total
  }
}

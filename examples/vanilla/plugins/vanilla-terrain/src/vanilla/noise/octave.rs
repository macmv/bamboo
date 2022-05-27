use super::{
  super::rng::{Rng, RngDeriver},
  Noise,
};

pub struct Octave<N> {
  // One for each octave. The tuple contains the noise function and the amplitude of that function.
  // If the sampler is None, the octave should be skipped.
  samplers:    Vec<(Option<N>, f64)>,
  lacunarity:  f64,
  persistence: f64,
}

impl<N: std::fmt::Debug> Octave<N> {
  pub fn new<R: Rng>(
    rng: &mut R,
    noise: impl Fn(&mut R) -> N,
    first_octave: i32,
    amplitudes: &[f64],
  ) -> Self {
    let mut samplers = Vec::with_capacity(amplitudes.len());

    let deriver = rng.create_deriver();
    for k in 0..amplitudes.len() {
      if amplitudes[k] == 0.0 {
        continue;
      };
      let mut rng = deriver.create_rng(&format!("octave_{}", first_octave + k as i32));
      samplers.push((Some(noise(&mut rng)), amplitudes[k]));
    }

    let lacunarity = 2.0_f64.powi(first_octave);
    let len = amplitudes.len() as i32;
    let persistence = 2.0_f64.powi(len - 1) / (2.0_f64.powi(len) - 1.0);
    // this.field_36632 = this.method_40557(2.0);

    Octave { samplers, lacunarity, persistence }
  }
  pub fn new_legacy_octaves<R: Rng>(
    rng: &mut R,
    noise: impl Fn(&mut R) -> N,
    octaves: &[i32],
  ) -> Self {
    assert!(!octaves.is_empty());
    let i = -octaves[0];
    let k = i + octaves[octaves.len() - 1] + 1;
    if k < 1 {
      panic!("total number of octaves needs to be >= 1");
    }
    let mut amplitudes = vec![0.0; k as usize];
    for octave in octaves {
      amplitudes[(octave + i) as usize] = 1.0;
    }

    Self::new_legacy(rng, noise, -i, &amplitudes)
  }
  pub fn new_legacy<R: Rng>(
    rng: &mut R,
    noise: impl Fn(&mut R) -> N,
    first_octave: i32,
    amplitudes: &[f64],
  ) -> Self {
    let len = amplitudes.len() as i32;
    let j = -first_octave;
    let mut samplers = vec![];
    samplers.resize_with(amplitudes.len(), || None);

    let first = noise(rng);
    if j >= 0 && j < len && amplitudes[j as usize] != 0.0 {
      samplers[j as usize] = Some(first);
    }
    for k in (0..j).rev() {
      // We still want to use the same number of rng.next() calls, even if we don't
      // store this perlin.
      let perlin = noise(rng);
      if k < amplitudes.len() as i32 {
        let amp = amplitudes[k as usize];
        if amp != 0.0 {
          samplers[k as usize] = Some(perlin);
        }
      }
    }
    if samplers.iter().filter(|s| s.is_some()).count()
      != amplitudes.iter().filter(|&&v| v != 0.0).count()
    {
      panic!("failed to create correct number of noise levels for given non-zero amplitudes");
    }
    if j < len - 1 {
      panic!("positive octaves are not allowed");
    }

    let lacunarity = 2.0_f64.powi(-j);
    let persistence = 2.0_f64.powi(len - 1) / (2.0_f64.powi(len) - 1.0);
    // this.field_36632 = this.method_40557(2.0);

    Octave {
      samplers: samplers.into_iter().enumerate().map(|(i, s)| (s, amplitudes[i])).collect(),
      lacunarity,
      persistence,
    }
  }
  pub fn get_octave(&self, octave: usize) -> &Option<N> {
    &self.samplers[self.samplers.len() - 1 - octave].0
  }
  pub fn octaves(&self) -> usize { self.samplers.len() }
}

pub fn maintain_precision(v: f64) -> f64 { v - (v / 3.3554432E7 + 0.5).floor() * 3.3554432E7 }
// pub fn maintain_precision(v: f64) -> f64 { v }

impl<N: Noise> Noise for Octave<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    let mut total = 0.0;
    let mut lacunarity = self.lacunarity;
    let mut persistence = self.persistence;
    for (noise, amplitude) in &self.samplers {
      if let Some(n) = noise {
        let value = n.sample(
          maintain_precision(x * lacunarity),
          maintain_precision(y * lacunarity),
          maintain_precision(z * lacunarity),
        );
        total += amplitude * value * persistence;
      }
      lacunarity *= 2.0;
      persistence /= 2.0;
    }
    total
  }
}

#[cfg(test)]
mod tests {
  use super::{
    super::{
      super::rng::{SimpleRng, Xoroshiro},
      assert_similar, Perlin,
    },
    *,
  };
  use pretty_assertions::assert_eq;

  #[test]
  fn octave_simple_test() {
    let mut rng = SimpleRng::new(0);
    let mut octave = Octave::new(&mut rng, |rng| Perlin::new(rng), 3, &[1.0, 2.0, 3.0]);

    assert_similar(octave.sample(0.0, 0.0, 0.0), -0.0974);
    assert_similar(octave.sample(0.5, 0.0, 0.0), 0.35774);
  }

  #[test]
  fn octave_xoroshiro_test() {
    let mut rng = Xoroshiro::new(0);
    let mut octave = Octave::new(&mut rng, |rng| Perlin::new(rng), 3, &[1.0, 2.0, 3.0]);

    assert_similar(octave.sample(0.0, 0.0, 0.0), -0.07138800152556417);
    assert_similar(octave.sample(0.5, 0.0, 0.0), 0.43152260160800854);
  }
}

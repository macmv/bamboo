use super::{
  super::rng::{Rng, RngDeriver},
  Noise,
};

pub struct Octave<N> {
  // One for each octave. The tuple contains the noise function and the amplitude of that function.
  samplers:    Vec<(N, f64)>,
  lacunarity:  f64,
  persistence: f64,
}

impl<N> Octave<N> {
  pub fn new<R: Rng>(
    rng: &mut R,
    noise: impl Fn(&mut <<R as Rng>::Deriver as RngDeriver>::Rng) -> N,
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
      samplers.push((noise(&mut rng), amplitudes[k]));
    }

    let lacunarity = 2.0_f64.powi(first_octave);
    let len = amplitudes.len() as i32;
    let persistence = 2.0_f64.powi(len - 1) / (2.0_f64.powi(len) - 1.0);
    // this.field_36632 = this.method_40557(2.0);

    Octave { samplers, lacunarity, persistence }
  }
  pub fn new_legacy<R: Rng>() -> Self {
    /*
    this.firstOctave = pair.getFirst();
    this.amplitudes = pair.getSecond();
    int i = this.amplitudes.size();
    int j = -this.firstOctave;
    this.octaveSamplers = new PerlinNoiseSampler[i];

    double d;
    PerlinNoiseSampler perlinNoiseSampler = new PerlinNoiseSampler(random);
    if (j >= 0 && j < i && (d = this.amplitudes.getDouble(j)) != 0.0) {
      this.octaveSamplers[j] = perlinNoiseSampler;
    }
    for (int k = j - 1; k >= 0; --k) {
      if (k < i) {
        double e = this.amplitudes.getDouble(k);
        if (e != 0.0) {
          this.octaveSamplers[k] = new PerlinNoiseSampler(random);
          continue;
        }
        OctavePerlinNoiseSampler.skipCalls(random);
        continue;
      }
      OctavePerlinNoiseSampler.skipCalls(random);
    }
    if (Arrays.stream(this.octaveSamplers).filter(Objects::nonNull).count() != this.amplitudes.stream().filter(double_ -> double_ != 0.0).count()) {
      panic!("failed to create correct number of noise levels for given non-zero amplitudes");
    }
    if (j < i - 1) {
      panic!("positive octaves are not allowed");
    }

    this.lacunarity = Math.pow(2.0, -j);
    this.persistence = Math.pow(2.0, i - 1) / (Math.pow(2.0, i) - 1.0);
    this.field_36632 = this.method_40557(2.0);
    */
    todo!()
  }
  pub fn get_octave(&self, octave: usize) -> &N {
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

#[cfg(test)]
mod tests {
  use super::{
    super::{super::rng::SimpleRng, Perlin},
    *,
  };
  use pretty_assertions::assert_eq;

  #[test]
  fn single_perlin_test() {
    let mut rng = SimpleRng::new(0);
    let mut octave = Octave::new(&mut rng, |rng| Perlin::new(rng), 3, &[1.0, 2.0, 3.0]);

    assert_similar(octave.sample(0.0, 0.0, 0.0), -0.0974);
    assert_similar(octave.sample(0.5, 0.0, 0.0), 0.35774);
  }

  #[track_caller]
  fn assert_similar(actual: f64, expected: f64) {
    if (expected - actual).abs() > 0.0001 {
      panic!("Expected: {expected}, got: {actual}");
    }
  }
}

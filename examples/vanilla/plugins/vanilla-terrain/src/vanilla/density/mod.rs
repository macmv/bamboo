use super::{
  noise::{Cached, CachedDoublePerlin, DoublePerlin, Noise, Octave, Perlin},
  noise_params::{self, NoiseParams},
  rng::{Rng, SimpleRng, Xoroshiro},
};
use float_ord::FloatOrd;
use std::sync::Arc;

mod cached;
mod interpolated;

pub use cached::DensityCached;
pub use interpolated::Interpolated;

pub type DensityFunc = DensityCached<Interpolated>;

pub struct World {
  pub density_funcs: DensityFuncs,
}

pub struct DensityFuncs {
  noise_funcs:       NoiseFuncs,
  pub shift_x:       Arc<Shift>,
  pub shift_z:       Arc<Shift>,
  pub continents:    Arc<Shifted>,
  pub final_density: Arc<DensityCached<Interpolated>>,
}

pub struct NoiseFuncs {
  offset:     Arc<CachedDoublePerlin>,
  continents: Arc<CachedDoublePerlin>,
}

pub struct NoiseConfig {
  pub xz_scale:  f64,
  pub y_scale:   f64,
  pub xz_factor: f64,
  pub y_factor:  f64,
}

impl NoiseFuncs {
  pub fn new<R: Rng>(rng: &mut R) -> Self {
    macro_rules! noise {
      ( $params:expr ) => {
        Arc::new(Cached::new(DoublePerlin::new(
          Octave::new(rng, |rng| Perlin::new(rng), -$params.first_octave, $params.amplitudes),
          Octave::new(rng, |rng| Perlin::new(rng), -$params.first_octave, $params.amplitudes),
          $params.amplitudes[0],
        )))
      };
    }
    NoiseFuncs {
      offset:     noise!(noise_params::OFFSET),
      continents: noise!(noise_params::CONTINENTALNESS),
    }
  }
}

impl DensityFuncs {
  pub fn new(noise: NoiseFuncs, rng: &mut impl Rng) -> Self {
    let shift_x = Arc::new(shift(noise.offset.clone()));
    let shift_z = Arc::new(shift(noise.offset.clone()));
    let continents =
      Arc::new(shifted(shift_x.clone(), shift_z.clone(), 0.25, noise.continents.clone()));

    let mut xoroshiro = Xoroshiro::new(0);
    let final_density = Arc::new(DensityCached::new(Interpolated::new(
      Octave::new_legacy_octaves(
        &mut xoroshiro,
        |rng| Cached::new(Perlin::new(rng)),
        &(-15..=0).collect::<Vec<_>>(),
      ),
      Octave::new_legacy_octaves(
        &mut xoroshiro,
        |rng| Cached::new(Perlin::new(rng)),
        &(-15..=0).collect::<Vec<_>>(),
      ),
      Octave::new_legacy_octaves(
        &mut xoroshiro,
        |rng| Cached::new(Perlin::new(rng)),
        &(-7..=0).collect::<Vec<_>>(),
      ),
      4,
      8,
      &NoiseConfig { xz_scale: 1.0, y_scale: 1.0, xz_factor: 80.0, y_factor: 160.0 },
    )));
    DensityFuncs { noise_funcs: noise, shift_x, shift_z, continents, final_density }
  }
}

impl World {
  pub fn new(rng: &mut impl Rng) -> Self {
    let noise_funcs = NoiseFuncs::new(rng);
    let density_funcs = DensityFuncs::new(noise_funcs, rng);
    World { density_funcs }
  }
  pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    self.density_funcs.continents.sample(NoisePos { x: x as i32, y: y as i32, z: z as i32 })
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NoisePos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

pub trait Density {
  fn sample(&self, pos: NoisePos) -> f64;
}

pub struct Shift {
  noise: Arc<CachedDoublePerlin>,
}

pub struct Shifted {
  xz_scale: f64,
  y_scale:  f64,
  shift_x:  Arc<Shift>,
  shift_z:  Arc<Shift>,
  noise:    Arc<Cached<DoublePerlin>>,
}

impl Density for Shifted {
  fn sample(&self, pos: NoisePos) -> f64 {
    let d = (pos.x as f64) * self.xz_scale + self.shift_x.sample(pos);
    let e = (pos.y as f64) * self.y_scale;
    let f = (pos.z as f64) * self.xz_scale + self.shift_z.sample(pos);
    return self.noise.sample(d, e, f);
  }
}
impl Density for Shift {
  fn sample(&self, pos: NoisePos) -> f64 {
    let d = pos.x as f64;
    let e = 0.0;
    let f = pos.z as f64;
    return self.noise.sample(d * 0.25, e * 0.25, f * 0.25) * 4.0;
  }
}

pub fn shift(noise: Arc<CachedDoublePerlin>) -> Shift { Shift { noise } }

pub fn shifted(
  shift_x: Arc<Shift>,
  shift_z: Arc<Shift>,
  xz_scale: f64,
  noise: Arc<CachedDoublePerlin>,
) -> Shifted {
  Shifted { xz_scale, y_scale: 0.0, shift_x, shift_z, noise }
}

use super::{
  noise::{DoublePerlin, Noise, Octave, OctavePerlin, Perlin},
  noise_params::{self, NoiseParams},
  rng::Rng,
};
use std::sync::Arc;

pub struct World {
  density_funcs: DensityFuncs,
}

pub struct DensityFuncs {
  noise_funcs: NoiseFuncs,
  shift_x:     Arc<Shift>,
  shift_z:     Arc<Shift>,
  continents:  Arc<Shifted>,
}

pub struct NoiseFuncs {
  offset:     Arc<DoublePerlin>,
  continents: Arc<DoublePerlin>,
}

impl NoiseFuncs {
  pub fn new(rng: &mut Rng) -> Self {
    macro_rules! noise {
      ( $params:expr ) => {
        Arc::new(DoublePerlin::new(
          Octave::new(Perlin::new(rng), -$params.first_octave),
          Octave::new(Perlin::new(rng), -$params.first_octave),
        ))
      };
    }
    NoiseFuncs {
      offset:     noise!(noise_params::OFFSET),
      continents: noise!(noise_params::CONTINENTALNESS),
    }
  }
}

impl DensityFuncs {
  pub fn new(noise: NoiseFuncs, rng: &mut Rng) -> Self {
    let shift_x = Arc::new(shift(noise.offset.clone()));
    let shift_z = Arc::new(shift(noise.offset.clone()));
    let continents =
      Arc::new(shifted(shift_x.clone(), shift_z.clone(), 0.25, noise.continents.clone()));
    DensityFuncs { shift_x, shift_z, continents, noise_funcs: noise }
  }
}

impl World {
  pub fn new(rng: &mut Rng) -> Self {
    let noise_funcs = NoiseFuncs::new(rng);
    let density_funcs = DensityFuncs::new(noise_funcs, rng);
    World { density_funcs }
  }
  pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    self.density_funcs.continents.sample(NoisePos { x: x as i32, y: y as i32, z: z as i32 })
  }
}

#[derive(Clone, Copy)]
pub struct NoisePos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

pub trait Density {
  fn sample(&self, pos: NoisePos) -> f64;
}

pub struct Shift {
  noise: Arc<DoublePerlin>,
}

pub struct Shifted {
  xz_scale: f64,
  y_scale:  f64,
  shift_x:  Arc<Shift>,
  shift_z:  Arc<Shift>,
  noise:    Arc<DoublePerlin>,
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

pub fn shift(noise: Arc<DoublePerlin>) -> Shift { Shift { noise } }

pub fn shifted(
  shift_x: Arc<Shift>,
  shift_z: Arc<Shift>,
  xz_scale: f64,
  noise: Arc<DoublePerlin>,
) -> Shifted {
  Shifted { xz_scale, y_scale: 0.0, shift_x, shift_z, noise }
}

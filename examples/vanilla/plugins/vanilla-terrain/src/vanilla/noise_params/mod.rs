/*
mod base_3d_noise;
pub use base_3d_noise::Base3DNoise;
*/

pub static OFFSET: NoiseParams = NoiseParams::new(-9, &[1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0]);
pub static CONTINENTALNESS: NoiseParams =
  NoiseParams::new(-9, &[2.0, 3.0, 3.0, 3.0, 2.0, 2.0, 2.0, 2.0]);

#[derive(Clone, Copy)]
pub struct NoiseParams {
  pub first_octave: i32,
  pub amplitudes:   &'static [f64],
}

impl NoiseParams {
  pub const fn new(first_octave: i32, amplitudes: &'static [f64]) -> Self {
    NoiseParams { first_octave, amplitudes }
  }
}

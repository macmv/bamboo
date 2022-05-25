mod cache;
mod cached;
mod double;
mod interpolated;
mod octave;
mod perlin;
mod wyhash;

pub use cache::Cache;
pub use cached::Cached;
pub use double::Double;
pub use interpolated::Interpolated;
pub use octave::Octave;
pub use perlin::Perlin;
pub use wyhash::{WyHash, WyHashBuilder};

pub type DoublePerlin = Double<Octave<Perlin>>;
pub type OctavePerlin = Octave<Perlin>;
pub type CachedDoublePerlin = Cached<DoublePerlin>;

pub struct NoiseConfig {
  pub xz_scale:  f64,
  pub y_scale:   f64,
  pub xz_factor: f64,
  pub y_factor:  f64,
}

pub trait Noise {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64;
  fn sample_scale(&self, x: f64, y: f64, z: f64, scale_y: f64, max_y: f64) -> f64 {
    self.sample(x, y, z)
  }
}

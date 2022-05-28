use super::{Cache, Noise};
use float_ord::FloatOrd;
use parking_lot::Mutex;
use std::ops::Deref;

#[derive(Debug)]
pub struct Cached<N> {
  noise: N,
  cache: Mutex<Option<(f64, f64, f64, f64, f64, f64)>>,
}

impl<N> Cached<N> {
  pub fn new(noise: N) -> Self { Cached { noise, cache: Mutex::new(None) } }
}

impl<N: Noise> Noise for Cached<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 { self.sample_scale(x, y, z, 0.0, 0.0) }
  fn sample_scale(&self, x: f64, y: f64, z: f64, y_scale: f64, y_max: f64) -> f64 {
    let mut lock = self.cache.lock();
    if let Some((c_x, c_y, c_z, c_ys, c_ym, val)) = *lock {
      if x == c_x && y == c_y && z == c_z && c_ys == y_scale && c_ym == y_max {
        return val;
      }
    }
    let val = self.noise.sample_scale(x, y, z, y_scale, y_max);
    *lock = Some((x, y, z, y_scale, y_max, val));
    val
  }
}

impl<N> From<N> for Cached<N> {
  fn from(n: N) -> Self { Cached::new(n) }
}

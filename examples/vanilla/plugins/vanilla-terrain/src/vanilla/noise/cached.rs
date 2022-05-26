use super::{Cache, Noise};
use float_ord::FloatOrd;
use parking_lot::Mutex;
use std::ops::Deref;

pub struct Cached<N> {
  noise: N,
  cache: Mutex<Option<(f64, f64, f64, f64)>>,
}

impl<N: Noise + Send + Sync + 'static> Cached<N> {
  pub fn new(noise: N) -> Self { Cached { noise, cache: Mutex::new(None) } }
}

impl<N: Noise> Noise for Cached<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    let mut lock = self.cache.lock();
    if let Some((c_x, c_y, c_z, val)) = *lock {
      if x == c_x && y == c_y && z == c_z {
        return val;
      }
    }
    let val = self.noise.sample(x, y, z);
    *lock = Some((x, y, z, val));
    val
  }
}

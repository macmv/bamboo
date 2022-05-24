use super::{Cache, Noise};
use float_ord::FloatOrd;
use parking_lot::Mutex;

pub struct Cached<N> {
  noise: N,
  cache: Mutex<Cache<(FloatOrd<f64>, FloatOrd<f64>, FloatOrd<f64>), f64>>,
}

impl<N: Noise + Send + Sync + 'static> Cached<N> {
  pub fn new(noise: N) -> Self { Cached { noise, cache: Mutex::new(Cache::new()) } }
}

impl<N: Noise> Noise for Cached<N> {
  fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
    *self.cache.lock().get(
      (FloatOrd(x), FloatOrd(y), FloatOrd(z)),
      |(x, y, z): (FloatOrd<_>, FloatOrd<_>, FloatOrd<_>)| self.noise.sample(x.0, y.0, z.0),
    )
  }
}

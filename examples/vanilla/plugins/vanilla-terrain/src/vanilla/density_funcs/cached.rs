use super::{super::noise::Cache, Density, NoisePos};
use parking_lot::Mutex;

pub struct DensityCached<D> {
  density: D,
  cache:   Mutex<Option<(NoisePos, f64)>>,
}

impl<D: Density + Send + Sync + 'static> DensityCached<D> {
  pub fn new(density: D) -> Self { DensityCached { density, cache: Mutex::new(None) } }
}

impl<D: Density> Density for DensityCached<D> {
  fn sample(&self, pos: NoisePos) -> f64 {
    let mut lock = self.cache.lock();
    if let Some((cached_pos, val)) = *lock {
      if pos == cached_pos {
        return val;
      }
    }
    let val = self.density.sample(pos);
    *lock = Some((pos, val));
    val
  }
}

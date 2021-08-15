use common::math::Voronoi;
use noise::{BasicMulti, MultiFractal, NoiseFn, Seedable};

/// This is a voronoi map, but all the input coordinates are shifted by two
/// noise maps.
pub struct WarpedVoronoi {
  map: Voronoi,
  x:   BasicMulti,
  y:   BasicMulti,
}

impl WarpedVoronoi {
  pub fn new(seed: u64) -> Self {
    WarpedVoronoi {
      map: Voronoi::new(seed),
      x:   BasicMulti::new().set_octaves(5).set_seed(seed as u32),
      y:   BasicMulti::new().set_octaves(5).set_seed((seed >> 32) as u32),
    }
  }

  pub fn get(&self, x: i32, y: i32) -> u32 {
    let new_x = x + (self.x.get([x as f64 / 100.0, y as f64 / 100.0]) * 20.0) as i32;
    let new_y = y + (self.y.get([x as f64 / 100.0, y as f64 / 100.0]) * 20.0) as i32;
    self.map.get(new_x, new_y)
  }
}

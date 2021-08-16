use super::PointGrid;

/// This is an infinitley expanding voronoi map. It returns a unique id for
/// every region that is retrieved. It should be used to choose which biome to
/// generate at each block.
pub struct Voronoi {
  grid: PointGrid,
}

impl Voronoi {
  pub fn new(seed: u64) -> Self {
    Voronoi { grid: PointGrid::new(seed, 256, 128) }
  }

  pub fn get(&self, x: i32, y: i32) -> u32 {
    let (px, py) = self.grid.closest_point(x, y);
    (px ^ py) as u32
  }
  pub fn dist_to_center(&self, x: i32, y: i32) -> f64 {
    let (px, py) = self.grid.closest_point(x, y);
    (((px - x).pow(2) + (py - y).pow(2)) as f64).sqrt()
  }
}

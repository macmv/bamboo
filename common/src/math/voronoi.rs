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
    ((px - x).pow(2) as f64 + (py - y).pow(2) as f64).sqrt()
  }
  /// Returns the closest neighbor of the given point. This is the second
  /// closest point to (x, y).
  pub fn closest_neighbor(&self, x: i32, y: i32) -> (i32, i32) {
    self.grid.neighbors(x, y)[1]
  }
  /// Returns the distance to the border of the region that (x, y) is in.
  pub fn dist_to_border(&self, x: i32, y: i32) -> f64 {
    let (nx, ny) = self.closest_neighbor(x, y);
    // TODO: Fix this to actually use borders
    ((nx - x).pow(2) as f64 + (ny - y).pow(2) as f64).sqrt()
  }
}

use super::{Point, PointGrid};

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

  pub fn get(&self, p: Point) -> u32 {
    let p = self.grid.closest_point(p);
    ((p.x as u32) ^ ((p.y as u32) << 16)) as u32
  }
  pub fn dist_to_center(&self, p: Point) -> f64 {
    self.grid.closest_point(p).dist(p)
  }
  /// Returns the closest neighbor of the given point. This is the second
  /// closest point to (x, y).
  pub fn closest_neighbor(&self, p: Point) -> Point {
    self.grid.neighbors(p)[1]
  }
  /// Returns the distance to the border of the region that (x, y) is in.
  pub fn dist_to_border(&self, p: Point) -> f64 {
    self.closest_neighbor(p).dist(p)
  }
}

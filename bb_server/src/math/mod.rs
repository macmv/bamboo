mod aabb;
mod point;
mod point_grid;
mod vec;
mod voronoi;

pub use aabb::{CollisionResult, AABB};
pub use point::{Pdope, Point, Vector};
pub use point_grid::PointGrid;
pub use vec::Vec3;
pub use voronoi::Voronoi;

use noise::{BasicMulti, MultiFractal, NoiseFn, Seedable, Perlin};

/// This is a voronoi map, but all the input coordinates are shifted by two
/// noise maps.
pub struct WarpedVoronoi {
  map: Voronoi,
  x:   BasicMulti<Perlin>,
  y:   BasicMulti<Perlin>,
}

impl WarpedVoronoi {
  pub fn new(seed: u64) -> Self {
    WarpedVoronoi {
      map: Voronoi::new(seed),
      x:   BasicMulti::<Perlin>::default().set_octaves(5).set_seed(seed as u32),
      y:  BasicMulti::<Perlin>::default().set_octaves(5).set_seed((seed >> 32) as u32),
    }
  }

  pub fn warp(&self, p: Point) -> Point {
    let new_x = p.x + (self.x.get([p.x as f64 / 100.0, p.y as f64 / 100.0]) * 20.0) as i32;
    let new_y = p.y + (self.y.get([p.x as f64 / 100.0, p.y as f64 / 100.0]) * 20.0) as i32;
    Point::new(new_x, new_y)
  }

  pub fn get(&self, p: Point) -> u32 { self.map.get(self.warp(p)) }
  // pub fn dist_to_center(&self, p: Point) -> f64 {
  //   self.map.dist_to_center(self.warp(p))
  // }
  pub fn dist_to_border(&self, p: Point) -> f64 { self.map.dist_to_border(self.warp(p)) }
}

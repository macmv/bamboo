use super::{Point, PointGrid, Vector};

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
  pub fn dist_to_border(&self, p: Point) -> f64 {
    let b = self.border(p);
    p.to_vec().dist(b)
  }
  /// Returns the closest point on the border of the region that (x, y) is in.
  pub fn border(&self, p: Point) -> Vector {
    let neighbors = self.grid.neighbors(p);
    let center = neighbors[0];
    let neighbor = neighbors[1];

    // We have something like this:
    //
    // `C` - Center
    // `N` - Neighbor
    // `|` - Border
    // B - A point on the border, specifically the average between C and N
    // T - The target point, that will be returned.
    //
    //        |
    //        |
    //  C --- B --- N
    //        |
    //        |
    //    P - T
    //        |
    //
    // To solve for the intersection between lines, desmos is very helpful, and I
    // solved for this:
    //
    // x = (S * a.x - a.y - I * b.x + b.y) / (S - I)
    // y = S(x - a.x) + a.y
    //
    // where S -> slope, I -> inverted slope, and a and b are the two points (in our
    // case they will be P and B).
    //
    // See: https://www.desmos.com/calculator/z2fjrcbb12

    // This is the slope between C and N, which is also the slope between P and T
    // (which is what we are looking for).
    let s = (center - neighbor).slope();
    // This is the slope between B and T.
    let i = s.perp().val();
    let s = s.val();

    let p = p.to_vec();
    let b = center.avg(neighbor);

    let x = (s * p.x - p.y - i * b.x + b.y) / (s - i);
    let y = s * (x - p.x) + p.y;

    Vector::new(x, y)
  }
}

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
    self.grid.neighbors(p, 2)[1]
  }
  pub fn dist_to_border(&self, p: Point) -> f64 {
    let mut min_dist = 1000.0;
    for b in self.borders(p) {
      let dist = p.to_vec().dist(b);
      if dist < min_dist {
        min_dist = dist;
      }
    }
    min_dist
  }
  /// Returns all the possible border points of the region that p is in. Some of
  /// the points may not be valid.
  pub fn borders(&self, p: Point) -> Vec<Vector> {
    let neighbors = self.grid.neighbors(p, 2);
    let center = neighbors[0];
    let mut out = vec![];
    for neighbor in neighbors {
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
      if (center - neighbor).y == 0 {
        // C -- N is vertical, so we have a horizontal border
        let x = center.avg(neighbor).x;
        let y = p.y;
        out.push(Vector::new(x, y.into()));
      } else if (center - neighbor).x == 0 {
        // C -- N is horizontal, so we have a vertical border
        let x = p.x;
        let y = center.avg(neighbor).y;
        out.push(Vector::new(x.into(), y));
      } else {
        let s = (center - neighbor).slope();
        // This is the slope between B and T, and we know it will not be NAN from the
        // check above.
        let i = s.perp().val();
        let s = s.val();

        let p = p.to_vec();
        let b = center.avg(neighbor);

        let x = (s * p.x - p.y - i * b.x + b.y) / (s - i);
        let y = s * (x - p.x) + p.y;

        out.push(Vector::new(x, y));
      }
    }
    out
  }
}

use super::{super::WyhashRng, Point};
use rand_core::RngCore;

/// This is a randomized point grid. It is built in such a way that the points
/// inside should be scattered in a random-looking fassion. This should be used
/// to spawn trees in the world.
#[derive(Debug)]
pub struct PointGrid {
  square_size: u32,
  points:      Vec<Vec<(u32, u32)>>,
}

impl PointGrid {
  /// Creates a new random point grid. `size` is the width and height of the
  /// grid of points. `square_size` is the size of each square in the point
  /// grid. So the total size of the grid is `size` * `square_size`.
  pub fn new(seed: u64, size: u32, square_size: u32) -> Self {
    let mut points = vec![vec![(0, 0); size as usize]; size as usize];
    let mut rng = WyhashRng::new(seed);
    for row in points.iter_mut() {
      for p in row.iter_mut() {
        let num = rng.next_u64();
        p.0 = num as u32 % square_size;
        p.1 = (num >> 32) as u32 % square_size;
      }
    }
    Self { square_size, points }
  }

  /// Returns true if there is a point at that position. The coordinates will be
  /// wrapped around the grid.
  pub fn contains(&self, p: Point) -> bool {
    let (rel, lookup) = self.normalize(p);
    let p = self.points[lookup.y as usize][lookup.x as usize];
    p.0 == rel.x as u32 && p.1 == rel.y as u32
  }

  /// Returns the closest point to the given point.
  pub fn closest_point(&self, p: Point) -> Point {
    self.neighbors(p, 1)[0]
  }

  /// Returns the neighbors of the given point. This list is sorted by distance
  /// to `p`.
  ///
  /// The radius is a distance in grid squares. So a value of 1 will give 9
  /// points, a value of 2 will give 25 points, etc.
  pub fn neighbors(&self, p: Point, radius: i32) -> Vec<Point> {
    let s = self.square_size as i32;
    let mut points = vec![];
    for x in -radius..=radius {
      for y in -radius..=radius {
        points.push(self.get(p + Point::new(s * x, s * y)));
      }
    }
    points.sort_by(|a, b| {
      let dist_a = p.dist(*a);
      let dist_b = p.dist(*b);
      dist_a.partial_cmp(&dist_b).unwrap()
    });
    points
  }

  // Takes two absolute coordinates for a point, and retrieves the point in
  // that square in absolute coordinate form.
  fn get(&self, p: Point) -> Point {
    let (_, lookup) = self.normalize(p);
    let inner = self.points[lookup.y as usize][lookup.x as usize];
    let x = p.x / self.square_size as i32;
    let y = p.y / self.square_size as i32;
    Point::new(
      inner.0 as i32 + x * self.square_size as i32,
      inner.1 as i32 + y * self.square_size as i32,
    )
  }

  /// Takes a user-passed coordinate, and returns the relative point, along
  /// with the x and y indicies to use to lookup the point.
  ///
  /// Both points will always have positive x and y values.
  fn normalize(&self, p: Point) -> (Point, Point) {
    let rel = p.pos_mod(self.square_size as i32);
    let len = self.points.len() as i32;
    let lookup = p.pos_div(self.square_size as i32).pos_mod(len);
    (rel, lookup)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normalize() {
    let g = PointGrid { square_size: 5, points: vec![vec![], vec![], vec![], vec![]] };
    assert_eq!(g.normalize(Point::new(1, 3)), (Point::new(1, 3), Point::new(0, 0)));
    assert_eq!(g.normalize(Point::new(7, 2)), (Point::new(2, 2), Point::new(1, 0)));
    assert_eq!(g.normalize(Point::new(4, 3)), (Point::new(4, 3), Point::new(0, 0)));
    assert_eq!(g.normalize(Point::new(-1, 3)), (Point::new(4, 3), Point::new(3, 0)));
  }
  #[test]
  fn test_contains() {
    let g = PointGrid {
      square_size: 5,
      points:      vec![
        vec![(1, 1), (0, 0), (0, 0)],
        vec![(3, 4), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
      ],
    };
    dbg!(g.normalize(Point::new(1, 1)));
    assert!(g.contains(Point::new(1, 1)));
    dbg!(g.normalize(Point::new(3, 9)));
    assert!(g.contains(Point::new(3, 9)));
  }
}

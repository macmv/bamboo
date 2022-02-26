use super::Point;
use sc_common::math::{RngCore, WyhashRng};

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
  pub fn closest_point(&self, p: Point) -> Point { self.neighbors(p, 2)[0] }

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
    let p = p.pos_div(self.square_size as i32);
    Point::new(
      inner.0 as i32 + p.x * self.square_size as i32,
      inner.1 as i32 + p.y * self.square_size as i32,
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
    assert_eq!(g.normalize(Point::new(13, 8)), (Point::new(3, 3), Point::new(2, 1)));
    assert_eq!(g.normalize(Point::new(-1, 3)), (Point::new(4, 3), Point::new(3, 0)));
  }
  #[test]
  fn test_get() {
    let g = PointGrid {
      square_size: 5,
      points:      vec![
        vec![(1, 1), (0, 0), (0, 0)],
        vec![(3, 4), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
      ],
    };
    for x in 0..3 * 5 {
      for y in 0..3 * 5 {
        let lookup_x = x / 5;
        let lookup_y = y / 5;
        if lookup_x == 0 && lookup_y == 0 {
          assert_eq!(g.get(Point::new(x, y)), Point::new(1, 1));
        } else if lookup_x == 0 && lookup_y == 1 {
          assert_eq!(g.get(Point::new(x, y)), Point::new(3, 4 + 5));
        } else {
          assert_eq!(g.get(Point::new(x, y)), Point::new(lookup_x * 5, lookup_y * 5));
          assert_eq!(
            g.get(Point::new(x - 3 * 5, y - 3 * 5)),
            Point::new((lookup_x - 3) * 5, (lookup_y - 3) * 5)
          );
        }
      }
    }
  }
  #[test]
  fn test_neighbors() {
    let g = PointGrid {
      square_size: 4,
      points:      vec![
        vec![(0, 0), (2, 3), (1, 3)],
        vec![(3, 1), (2, 2), (3, 2)],
        vec![(2, 1), (1, 2), (1, 1)],
      ],
    };
    // 11 | 21 12
    // ----------
    // 13 | 00 23
    // 32 | 31 22
    //
    // So, we should expect these points:
    // -3-3 | 2-3 5-2
    // --------------
    // -1 3 | 0 0 6 3
    // -3 6 | 3 5 6 6
    //
    // But, we don't get those at all, and I have no idea why.
    for p in g.neighbors(Point::new(0, 0), 1) {
      dbg!(p);
      let rel_x = ((p.x % 4) + 4) % 4;
      let rel_y = ((p.y % 4) + 4) % 4;
      let lookup_x = if p.x < 0 { 2 } else { p.x / 4 };
      let lookup_y = if p.y < 0 { 2 } else { p.y / 4 };
      let expected_rel = g.points[lookup_y as usize][lookup_x as usize];
      dbg!(rel_x, rel_y, lookup_x, lookup_y);
      assert_eq!((rel_x as u32, rel_y as u32), expected_rel);
    }
  }
  #[test]
  fn test_closest_point() {
    let g = PointGrid {
      square_size: 4,
      points:      vec![
        vec![(0, 0), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
      ],
    };
    for x in 0..3 * 4 {
      for y in 0..3 * 4 {
        let lookup_x = (x + 1) / 4;
        let lookup_y = (y + 1) / 4;
        assert_eq!(g.closest_point(Point::new(x, y)), Point::new(lookup_x * 4, lookup_y * 4));
      }
    }
    for x in 0..3 * 4 {
      for y in 0..3 * 4 {
        let lookup_x = (x + 1) / 4;
        let lookup_y = (y + 1) / 4;
        assert_eq!(
          g.closest_point(Point::new(x - 3 * 4, y - 3 * 4)),
          Point::new((lookup_x - 3) * 4, (lookup_y - 3) * 4)
        );
      }
    }

    let g = PointGrid {
      square_size: 5,
      points:      vec![
        vec![(0, 0), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
      ],
    };
    for x in 0..3 * 5 {
      for y in 0..3 * 5 {
        let lookup_x = (x + 2) / 5;
        let lookup_y = (y + 2) / 5;
        assert_eq!(g.closest_point(Point::new(x, y)), Point::new(lookup_x * 5, lookup_y * 5));
      }
    }

    let g = PointGrid {
      square_size: 8,
      points:      vec![
        vec![(0, 0), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
        vec![(0, 0), (0, 0), (0, 0)],
      ],
    };
    for x in 0..3 * 8 {
      for y in 0..3 * 8 {
        let lookup_x = (x + 3) / 8;
        let lookup_y = (y + 3) / 8;
        assert_eq!(g.closest_point(Point::new(x, y)), Point::new(lookup_x * 8, lookup_y * 8));
      }
    }

    let g =
      PointGrid { square_size: 4, points: vec![vec![(2, 2), (1, 1)], vec![(3, 0), (3, 3)]] };

    let expected_str = "
    ZZ ZZ AA AA BB BB WW WW
    ZZ AA AA AA BB .. BB WW
    AA AA .. AA BB BB BB BB
    AA AA AA CC CC BB BB BB
    AA CC CC .. CC CC CC DD
    XX CC CC CC CC CC DD DD
    XX XX CC CC CC DD DD DD
    XX XX XX YY YY DD DD ..";
    let mut expected = vec![vec![(0, 0); 8]; 8];
    let mut y = 0;
    for l in expected_str.lines() {
      if l == "" {
        continue;
      }
      for (x, s) in l.trim().split(" ").enumerate() {
        match s {
          "AA" => expected[y][x] = (2 + 0, 2 + 0),
          "BB" => expected[y][x] = (1 + 4, 1 + 0),
          "CC" => expected[y][x] = (3 + 0, 0 + 4),
          "DD" => expected[y][x] = (3 + 4, 3 + 4),
          "XX" => expected[y][x] = (3 - 4, 3 + 4),
          "YY" => expected[y][x] = (1 + 4, 1 + 8),
          "ZZ" => expected[y][x] = (3 - 4, 3 - 4),
          "WW" => expected[y][x] = (3 + 4, 3 - 4),
          ".." => expected[y][x] = (x as i32, y as i32),
          _ => unreachable!(),
        }
      }
      y += 1;
    }
    for x in 0..2 * 4 {
      for y in 0..2 * 4 {
        let (expected_x, expected_y) = expected[y as usize][x as usize];
        dbg!(x, y, expected_x, expected_y);
        println!("{:?}", g.closest_point(Point::new(x, y)));
        assert_eq!(g.closest_point(Point::new(x, y)), Point::new(expected_x, expected_y));
      }
    }
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

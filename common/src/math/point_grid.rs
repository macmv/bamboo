use super::WyhashRng;
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
  pub fn contains(&self, x: i32, y: i32) -> bool {
    let (rx, ry, x, y) = self.normalize(x, y);
    let p = self.points[y as usize][x as usize];
    p.0 == rx && p.1 == ry
  }

  /// Returns the closest point to the given point.
  pub fn closest_point(&self, x: i32, y: i32) -> (i32, i32) {
    self.neighbors(x, y)[0]
  }

  /// Returns the neighbors of the given point. This list is sorted by distance
  /// to (x, y).
  pub fn neighbors(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
    let s = self.square_size as i32;
    let mut points = vec![
      self.get(x - s, y - s),
      self.get(x, y - s),
      self.get(x + s, y - s),
      self.get(x - s, y),
      self.get(x, y),
      self.get(x + s, y),
      self.get(x - s, y + s),
      self.get(x, y + s),
      self.get(x + s, y + s),
    ];
    points.sort_by(|(ax, ay), (bx, by)| {
      let dist_a = ((ax - x).pow(2) as f64 + (ay - y).pow(2) as f64).sqrt();
      let dist_b = ((bx - x).pow(2) as f64 + (by - y).pow(2) as f64).sqrt();
      dist_a.partial_cmp(&dist_b).unwrap()
    });
    points
  }

  // Takes two absolute coordinates for a point, and retrieves the point in
  // that square in absolute coordinate form.
  fn get(&self, x: i32, y: i32) -> (i32, i32) {
    let (_, _, px, py) = self.normalize(x, y);
    let p = self.points[py as usize][px as usize];
    let x = x / self.square_size as i32;
    let y = y / self.square_size as i32;
    (p.0 as i32 + x * self.square_size as i32, p.1 as i32 + y * self.square_size as i32)
  }

  /// Takes a user-passed coordinate, and returns the relative x and y, along
  /// with the x and y indicies to use to lookup the point.
  fn normalize(&self, x: i32, y: i32) -> (u32, u32, u32, u32) {
    let s = self.square_size as i32;
    let rx = ((x % s) + s) as u32 % self.square_size;
    let ry = ((y % s) + s) as u32 % self.square_size;
    let len = self.points.len() as i32;
    let x = (((x / self.square_size as i32) % len) + len) as u32 % self.points.len() as u32;
    let y = (((y / self.square_size as i32) % len) + len) as u32 % self.points.len() as u32;
    (rx, ry, x, y)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normalize() {
    let g = PointGrid { square_size: 5, points: vec![vec![], vec![], vec![], vec![]] };
    assert_eq!(g.normalize(1, 3), (1, 3, 0, 0));
    assert_eq!(g.normalize(7, 2), (2, 2, 1, 0));
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
    dbg!(g.normalize(1, 1));
    assert!(g.contains(1, 1));
    dbg!(g.normalize(3, 9));
    assert!(g.contains(3, 9));
  }
}

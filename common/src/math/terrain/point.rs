pub struct Point {
  x: i32,
  y: i32,
}

impl Point {
  pub fn new(x: i32, y: i32) -> Self {
    Point { x, y }
  }
  /// Returns the distance to ther other point. This is the same as
  /// `((ax - bx).pow(2) + (ay - by).pow(2)).sqrt()`.
  pub fn dist(&self, other: &Point) -> f64 {
    ((self.x - other.x).pow(2) as f64 + (self.y - other.y).pow(2) as f64).sqrt()
  }
}

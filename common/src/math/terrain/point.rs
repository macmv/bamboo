use std::ops::{Add, Div, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
  pub x: i32,
  pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
  pub x: f64,
  pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slope(f64);

impl Point {
  pub fn new(x: i32, y: i32) -> Self {
    Point { x, y }
  }
  /// Returns the distance to ther other point. This is the same as
  /// `((ax - bx).pow(2) + (ay - by).pow(2)).sqrt()`.
  pub fn dist(&self, other: Point) -> f64 {
    ((self.x - other.x).pow(2) as f64 + (self.y - other.y).pow(2) as f64).sqrt()
  }

  /// Returns the positive modulo of self.x and self.y % rem. This will get the
  /// correct values inside a chunk, as normal mod will give you negatives
  /// sometimes.
  pub fn pos_mod(&self, rem: i32) -> Point {
    let x = ((self.x % rem) + rem) % rem;
    let y = ((self.y % rem) + rem) % rem;
    Point::new(x, y)
  }

  /// Returns the correctly rounded value of self / rem. If self.x is -1, and
  /// rem is, say, 16, then the resulting X value will be -1, not 0 (which is
  /// what the normal division operator would give).
  pub fn pos_div(&self, rem: i32) -> Point {
    // This should work, but causes things to break horribly
    // let x;
    // let y;
    // if self.x < 0 {
    //   x = (self.x + 1) / rem - 1
    // } else {
    //   x = self.x / rem
    // }
    // if self.y < 0 {
    //   y = (self.y + 1) / rem - 1
    // } else {
    //   y = self.y / rem
    // }
    Point::new(self.x / rem, self.y / rem)
  }

  pub fn slope(&self) -> Slope {
    Slope(self.y as f64 / self.x as f64)
  }

  pub fn avg(&self, other: Point) -> Vector {
    Vector::new((self.x + other.x) as f64 / 2.0, (self.y + other.y) as f64 / 2.0)
  }
  pub fn to_vec(&self) -> Vector {
    Vector::new(self.x as f64, self.y as f64)
  }
}

impl Vector {
  pub fn new(x: f64, y: f64) -> Self {
    Vector { x, y }
  }
  pub fn dist(&self, other: Vector) -> f64 {
    ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
  }

  pub fn slope(&self) -> Slope {
    Slope(self.y as f64 / self.x as f64)
  }
}

impl Slope {
  pub fn perp(self) -> Slope {
    Slope(-1.0 / self.0)
  }
  pub fn to_vec(self) -> Vector {
    Vector::new(self.0.cos(), self.0.sin())
  }
  pub fn val(&self) -> f64 {
    self.0
  }
}

impl Add for Point {
  type Output = Point;

  fn add(self, other: Point) -> Point {
    Point::new(self.x + other.x, self.y + other.y)
  }
}

impl Sub for Point {
  type Output = Point;

  fn sub(self, other: Point) -> Point {
    Point::new(self.x - other.x, self.y - other.y)
  }
}

impl Div for Point {
  type Output = Point;

  fn div(self, other: Point) -> Point {
    Point::new(self.x / other.x, self.y / other.y)
  }
}

impl Add for Vector {
  type Output = Vector;

  fn add(self, other: Vector) -> Vector {
    Vector::new(self.x + other.x, self.y + other.y)
  }
}

impl Sub for Vector {
  type Output = Vector;

  fn sub(self, other: Vector) -> Vector {
    Vector::new(self.x - other.x, self.y - other.y)
  }
}

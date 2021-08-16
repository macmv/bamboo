use std::ops::{Add, Deref, Sub};

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

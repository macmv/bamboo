use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
  pub x: f64,
  pub y: f64,
  pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
  pub x: f64,
  pub y: f64,
}

impl Vec3 {
  pub fn new(x: f64, y: f64, z: f64) -> Self {
    Vec3 { x, y, z }
  }
  /// Returns the velocity in the packet format. This is `self.x * 8000`,
  /// because self.x is in blocks/tick.
  pub fn fixed_x(&self) -> i16 {
    (self.x * 8000.0) as i16
  }
  /// Returns the velocity in the packet format. This is `self.y * 8000`,
  /// because self.y is in blocks/tick.
  pub fn fixed_y(&self) -> i16 {
    (self.y * 8000.0) as i16
  }
  /// Returns the velocity in the packet format. This is `self.z * 8000`,
  /// because self.z is in blocks/tick.
  pub fn fixed_z(&self) -> i16 {
    (self.z * 8000.0) as i16
  }
}

impl Add for Vec2 {
  type Output = Vec2;

  fn add(self, other: Vec2) -> Vec2 {
    Vec2 { x: self.x + other.x, y: self.y + other.y }
  }
}
impl Sub for Vec2 {
  type Output = Vec2;

  fn sub(self, other: Vec2) -> Vec2 {
    Vec2 { x: self.x - other.x, y: self.y - other.y }
  }
}

impl Mul<f64> for Vec2 {
  type Output = Vec2;

  fn mul(self, fac: f64) -> Vec2 {
    Vec2 { x: self.x * fac, y: self.y * fac }
  }
}
impl Div<f64> for Vec2 {
  type Output = Vec2;

  fn div(self, fac: f64) -> Vec2 {
    Vec2 { x: self.x / fac, y: self.y / fac }
  }
}

impl Add for Vec3 {
  type Output = Vec3;

  fn add(self, other: Vec3) -> Vec3 {
    Vec3 { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
  }
}
impl Sub for Vec3 {
  type Output = Vec3;

  fn sub(self, other: Vec3) -> Vec3 {
    Vec3 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
  }
}

impl Mul<f64> for Vec3 {
  type Output = Vec3;

  fn mul(self, fac: f64) -> Vec3 {
    Vec3 { x: self.x * fac, y: self.y * fac, z: self.z * fac }
  }
}
impl Div<f64> for Vec3 {
  type Output = Vec3;

  fn div(self, fac: f64) -> Vec3 {
    Vec3 { x: self.x / fac, y: self.y / fac, z: self.z / fac }
  }
}
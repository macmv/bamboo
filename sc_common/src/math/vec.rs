use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
  pub x: f64,
  pub y: f64,
  pub z: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
  pub x: f64,
  pub y: f64,
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

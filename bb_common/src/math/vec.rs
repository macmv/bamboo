use crate::util::Face;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

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
  pub fn new(x: f64, y: f64, z: f64) -> Self { Vec3 { x, y, z } }
  /// Returns the velocity in the packet format. This is `self.x * 8000`,
  /// because self.x is in blocks/tick.
  pub fn fixed_x(&self) -> i16 { (self.x * 8000.0) as i16 }
  /// Returns the velocity in the packet format. This is `self.y * 8000`,
  /// because self.y is in blocks/tick.
  pub fn fixed_y(&self) -> i16 { (self.y * 8000.0) as i16 }
  /// Returns the velocity in the packet format. This is `self.z * 8000`,
  /// because self.z is in blocks/tick.
  pub fn fixed_z(&self) -> i16 { (self.z * 8000.0) as i16 }

  /// Returns the length of this vector, squared.
  pub fn len_squared(&self) -> f64 { self.x.powi(2) + self.y.powi(2) + self.z.powi(2) }

  /// Returns the length of this vector. If possible, prefer
  /// [`len_squared`](Self::len_squared).
  pub fn len(&self) -> f64 { self.len_squared().sqrt() }

  pub fn as_face(&self) -> Face {
    let xa = self.x.abs();
    let ya = self.y.abs();
    let za = self.z.abs();
    if xa > ya && xa > za {
      if self.x > 0.0 {
        Face::East
      } else {
        Face::West
      }
    } else if ya > xa && ya > za {
      if self.y > 0.0 {
        Face::Top
      } else {
        Face::Bottom
      }
    } else {
      if self.z > 0.0 {
        Face::South
      } else {
        Face::North
      }
    }
  }
  pub fn as_horz_face(&self) -> Face {
    let xa = self.x.abs();
    let za = self.z.abs();
    if xa > za {
      if self.x > 0.0 {
        Face::East
      } else {
        Face::West
      }
    } else {
      if self.z > 0.0 {
        Face::South
      } else {
        Face::North
      }
    }
  }
}

impl Add for Vec2 {
  type Output = Vec2;

  fn add(self, other: Vec2) -> Vec2 { Vec2 { x: self.x + other.x, y: self.y + other.y } }
}
impl Sub for Vec2 {
  type Output = Vec2;

  fn sub(self, other: Vec2) -> Vec2 { Vec2 { x: self.x - other.x, y: self.y - other.y } }
}

impl Mul<f64> for Vec2 {
  type Output = Vec2;

  fn mul(self, fac: f64) -> Vec2 { Vec2 { x: self.x * fac, y: self.y * fac } }
}
impl Div<f64> for Vec2 {
  type Output = Vec2;

  fn div(self, fac: f64) -> Vec2 { Vec2 { x: self.x / fac, y: self.y / fac } }
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
impl AddAssign for Vec3 {
  fn add_assign(&mut self, other: Vec3) {
    self.x += other.x;
    self.y += other.y;
    self.z += other.z;
  }
}
impl SubAssign for Vec3 {
  fn sub_assign(&mut self, other: Vec3) {
    self.x -= other.x;
    self.y -= other.y;
    self.z -= other.z;
  }
}

impl Mul<f64> for Vec3 {
  type Output = Vec3;

  fn mul(self, fac: f64) -> Vec3 { Vec3 { x: self.x * fac, y: self.y * fac, z: self.z * fac } }
}
impl Div<f64> for Vec3 {
  type Output = Vec3;

  fn div(self, fac: f64) -> Vec3 { Vec3 { x: self.x / fac, y: self.y / fac, z: self.z / fac } }
}

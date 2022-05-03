use super::{CollisionResult, AABB};
use bb_common::{math::FPos, util::Face};
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

impl Vec3 {
  /// Moves this `vec3` in the given direction, and make sure that it doesn't
  /// intersect with any of the given collision boxes.
  ///
  /// Returns `Some(result)` if this collided with anything.
  pub fn move_towards(&mut self, delta: Vec3, nearby: &[AABB]) -> Option<CollisionResult> {
    let mut result = None;
    let start = *self;
    let end = *self + delta;
    let mut time = 1.0;
    for &wall in nearby.iter().filter(|&&o| end.is_colliding_with(o)) {
      // Time to collide with the object, in each axis. We use this to find out which
      // axis will collide first.
      //
      // This two vectors store 6 numbers, each of which is a collision percentage. If
      // any of these collision percentages are within 0..1, then we have a potential
      // collision at that percentage.
      let t_min = Vec3::new(
        (wall.min_x() - start.x) / delta.x,
        (wall.min_y() - start.y) / delta.y,
        (wall.min_z() - start.z) / delta.z,
      );
      let t_max = Vec3::new(
        (wall.max_x() - start.x) / delta.x,
        (wall.max_y() - start.y) / delta.y,
        (wall.max_z() - start.z) / delta.z,
      );

      let mut axis = None;
      macro_rules! axis {
        ($time:expr, $axis:ident: $axis_val:expr, ($min_a:ident, $max_a:ident): $a:ident, ($min_b:ident, $max_b:ident): $b:ident) => {
          if $time.$axis == 0.0 {
            time = 0.0;
            axis = Some($axis_val);
          }
          if $time.$axis > 0.0 && $time.$axis < 1.0 && $time.$axis < time {
            let t = $time.$axis;
            let $a = start.$a + delta.$a * t;
            let $b = start.$b + delta.$b * t;
            if in_range($a, (wall.$min_a(), wall.$max_a()))
              && in_range($b, (wall.$min_b(), wall.$max_b()))
            {
              time = t;
              axis = Some($axis_val);
            }
          }
        };
      }

      axis!(t_min, x: Vec3::new(1.0, 0.0, 0.0), (min_y, max_y): y, (min_z, max_z): z);
      axis!(t_min, y: Vec3::new(0.0, 1.0, 0.0), (min_x, max_x): x, (min_z, max_z): z);
      axis!(t_min, z: Vec3::new(0.0, 0.0, 1.0), (min_x, max_x): x, (min_y, max_y): y);
      axis!(t_max, x: Vec3::new(1.0, 0.0, 0.0), (min_y, max_y): y, (min_z, max_z): z);
      axis!(t_max, y: Vec3::new(0.0, 1.0, 0.0), (min_x, max_x): x, (min_z, max_z): z);
      axis!(t_max, z: Vec3::new(0.0, 0.0, 1.0), (min_x, max_x): x, (min_y, max_y): y);

      if let Some(axis) = axis {
        result = Some(CollisionResult { axis, factor: time })
      }
    }

    if result.is_some() {
      *self += delta * time;
    } else {
      *self += delta;
    }
    result
  }

  /// Returns true if self is inside other. Being next to other
  /// (sides being equal) will return false.
  pub fn is_colliding_with(&self, other: AABB) -> bool {
    (self.x > other.min_x() && self.x < other.max_x())
      && (self.y > other.min_y() && self.y < other.max_y())
      && (self.z > other.min_z() && self.z < other.max_z())
  }
}

fn in_range(val: f64, range: (f64, f64)) -> bool {
  let (min, max) = range;
  val > min && val < max
}

impl From<Vec3> for FPos {
  fn from(v: Vec3) -> FPos { FPos::new(v.x, v.y, v.z) }
}
impl From<FPos> for Vec3 {
  fn from(v: FPos) -> Vec3 { Vec3::new(v.x, v.y, v.z) }
}

impl Add<Vec3> for FPos {
  type Output = Self;
  fn add(self, other: Vec3) -> Self {
    Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
  }
}
impl AddAssign<Vec3> for FPos {
  fn add_assign(&mut self, other: Vec3) {
    self.x += other.x;
    self.y += other.y;
    self.z += other.z;
  }
}

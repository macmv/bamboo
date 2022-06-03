use super::{ChunkPos, Pos};
use bb_macros::Transfer;
use std::{
  error::Error,
  fmt,
  ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

#[derive(Debug)]
pub struct FPosError {
  pos: FPos,
  msg: String,
}

impl fmt::Display for FPosError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid position: {} {}", self.pos, self.msg)
  }
}

impl Error for FPosError {}

#[derive(Transfer, Debug, Clone, Copy, PartialEq)]
pub struct FPos {
  pub x: f64,
  pub y: f64,
  pub z: f64,
}

impl fmt::Display for FPos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "FPos({} {} {})", self.x, self.y, self.z)
  }
}

impl Default for FPos {
  fn default() -> FPos { FPos::new(0.0, 0.0, 0.0) }
}

impl From<Pos> for FPos {
  fn from(p: Pos) -> FPos { FPos { x: p.x.into(), y: p.y.into(), z: p.z.into() } }
}

impl FPos {
  /// Creates a new block position. This can be used to find chunk coordinates,
  /// place blocks, or send a position in a packet.
  #[inline(always)]
  pub fn new(x: f64, y: f64, z: f64) -> Self { FPos { x, y, z } }
  /// Returns the X value of the position.
  #[inline(always)]
  pub fn x(&self) -> f64 { self.x }
  /// Returns the Y value of the position.
  #[inline(always)]
  pub fn y(&self) -> f64 { self.y }
  /// Returns the Z value of the position.
  #[inline(always)]
  pub fn z(&self) -> f64 { self.z }
  /// Returns the X value of the position, as a fixed precision float. This is
  /// the X position multiplied by 32. It is how position packets are sent on
  /// 1.8.
  #[inline(always)]
  pub fn fixed_x(&self) -> i32 { (self.x * 32.0) as i32 }
  /// Returns the Y value of the position, as a fixed precision float. This is
  /// the Y position multiplied by 32. It is how position packets are sent on
  /// 1.8.
  #[inline(always)]
  pub fn fixed_y(&self) -> i32 { (self.y * 32.0) as i32 }
  /// Returns the Z value of the position, as a fixed precision float. This is
  /// the Z position multiplied by 32. It is how position packets are sent on
  /// 1.8.
  #[inline(always)]
  pub fn fixed_z(&self) -> i32 { (self.z * 32.0) as i32 }
  /// Returns self, with x set to the given value.
  #[inline(always)]
  #[must_use = "with_x returns a modified version of self"]
  pub fn with_x(mut self, x: f64) -> Self {
    self.x = x;
    self
  }
  /// Returns self, with y set to the given value.
  #[inline(always)]
  #[must_use = "with_y returns a modified version of self"]
  pub fn with_y(mut self, y: f64) -> Self {
    self.y = y;
    self
  }
  /// Returns self, with z set to the given value.
  #[inline(always)]
  #[must_use = "with_z returns a modified version of self"]
  pub fn with_z(mut self, z: f64) -> Self {
    self.z = z;
    self
  }
  /// Returns self, with x set to self.x plus the given value.
  #[inline(always)]
  #[must_use = "add_x returns a modified version of self"]
  pub fn add_x(mut self, x: f64) -> Self {
    self.x += x;
    self
  }
  /// Returns self, with y set to self.y plus the given value.
  #[inline(always)]
  #[must_use = "add_y returns a modified version of self"]
  pub fn add_y(mut self, y: f64) -> Self {
    self.y += y;
    self
  }
  /// Returns self, with z set to self.z plus the given value.
  #[inline(always)]
  #[must_use = "add_z returns a modified version of self"]
  pub fn add_z(mut self, z: f64) -> Self {
    self.z += z;
    self
  }
  /// Returns the block that this position is in.
  #[inline(always)]
  pub fn block(&self) -> Pos {
    Pos::new(self.x.floor() as i32, self.y.floor() as i32, self.z.floor() as i32)
  }
  /// Returns the chunk that this position is in. This is the same as
  /// `self.block().chunk()`.
  #[inline(always)]
  pub fn chunk(&self) -> ChunkPos { self.block().chunk() }
  /// Creates a new error from this position. This should be used to signify
  /// that an invalid position was passed somewhere.
  pub fn err(&self, msg: String) -> FPosError { FPosError { pos: *self, msg } }

  /// Returns the distance to the other position.
  pub fn dist(&self, other: FPos) -> f64 {
    (((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)) as f64)
      .sqrt()
  }
  /// Returns the squared distance to the other position. Since block postitions
  /// are always ints, this will also always be exactly an int.
  pub fn dist_squared(&self, other: FPos) -> f64 {
    (self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)
  }

  /// Returns the minimum and maximum of each value of the three positions. The
  /// first argument returned is the min, and the second argument returned is
  /// the max.
  ///
  /// # Example
  ///
  /// ```
  /// # use bb_common::math::FPos;
  /// assert_eq!(
  ///   FPos::new(1.0, 5.0, 6.0).min_max(FPos::new(3.0, 3.0, 3.0)),
  ///   (FPos::new(1.0, 3.0, 3.0), FPos::new(3.0, 5.0, 6.0))
  /// );
  /// // different syntax, does the same thing
  /// assert_eq!(
  ///   FPos::min_max(FPos::new(1.0, 5.0, 6.0), FPos::new(3.0, 3.0, 3.0)),
  ///   (FPos::new(1.0, 3.0, 3.0), FPos::new(3.0, 5.0, 6.0))
  /// );
  /// ```
  pub fn min_max(self, other: FPos) -> (FPos, FPos) {
    (
      FPos::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z)),
      FPos::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z)),
    )
  }

  /// Returns the position with all values rounded down.
  pub fn floor(&self) -> FPos { FPos::new(self.x.floor(), self.y.floor(), self.z.floor()) }
  /// Returns the position with all values rounded up.
  pub fn ceil(&self) -> FPos { FPos::new(self.x.ceil(), self.y.ceil(), self.z.ceil()) }

  /// Returns the length of this vector.
  pub fn size(&self) -> f64 { (self.x.powi(2) + self.y.powi(2) + self.z.powi(2)).sqrt() }

  /// Returns the cross product of `self` and `other`. Order matters here.
  ///
  /// ```
  /// # use bb_common::math::FPos;
  /// assert_eq!(
  ///   FPos::new(1.0, 2.0, 3.0).cross(FPos::new(3.0, 2.0, 1.0)),
  ///   FPos::new(-4.0, 8.0, -4.0),
  /// );
  /// ```
  pub fn cross(self, other: FPos) -> FPos {
    FPos::new(
      (self.y * other.z) - (self.z * other.y),
      -((self.x * other.z) - (self.z * other.x)),
      (self.x * other.y) - (self.y * other.x),
    )
  }
}

impl Add for FPos {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
  }
}
impl AddAssign for FPos {
  fn add_assign(&mut self, other: Self) {
    self.x += other.x;
    self.y += other.y;
    self.z += other.z;
  }
}

impl Sub for FPos {
  type Output = Self;
  fn sub(self, other: Self) -> Self {
    Self { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
  }
}

impl SubAssign for FPos {
  fn sub_assign(&mut self, other: Self) {
    self.x -= other.x;
    self.y -= other.y;
    self.z -= other.z;
  }
}

impl Div<f64> for FPos {
  type Output = Self;
  fn div(self, other: f64) -> Self {
    Self { x: self.x / other, y: self.y / other, z: self.z / other }
  }
}
impl DivAssign<f64> for FPos {
  fn div_assign(&mut self, other: f64) {
    self.x /= other;
    self.y /= other;
    self.z /= other;
  }
}
impl Mul<f64> for FPos {
  type Output = Self;
  fn mul(self, other: f64) -> Self {
    Self { x: self.x * other, y: self.y * other, z: self.z * other }
  }
}
impl MulAssign<f64> for FPos {
  fn mul_assign(&mut self, other: f64) {
    self.x *= other;
    self.y *= other;
    self.z *= other;
  }
}

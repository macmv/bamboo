use super::{ChunkPos, Pos, Vec3};
use std::{
  error::Error,
  fmt,
  ops::{Add, AddAssign, Sub, SubAssign},
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

#[derive(Debug, Clone, Copy, PartialEq)]
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
  fn default() -> FPos {
    FPos::new(0.0, 0.0, 0.0)
  }
}

impl From<Pos> for FPos {
  fn from(p: Pos) -> FPos {
    FPos { x: p.x.into(), y: p.y.into(), z: p.z.into() }
  }
}
impl From<Vec3> for FPos {
  fn from(v: Vec3) -> FPos {
    FPos::new(v.x, v.y, v.z)
  }
}
impl From<FPos> for Vec3 {
  fn from(v: FPos) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
  }
}

impl FPos {
  /// Creates a new block position. This can be used to find chunk coordinates,
  /// place blocks, or send a position in a packet.
  pub fn new(x: f64, y: f64, z: f64) -> Self {
    FPos { x, y, z }
  }
  /// Returns the X value of the position.
  #[inline(always)]
  pub fn x(&self) -> f64 {
    self.x
  }
  /// Returns the Y value of the position.
  #[inline(always)]
  pub fn y(&self) -> f64 {
    self.y
  }
  /// Returns the Z value of the position.
  #[inline(always)]
  pub fn z(&self) -> f64 {
    self.z
  }
  /// Returns the X value of the position, as a fixed precision float. This is
  /// the X position multiplied by 32. It is how position packets are sent on
  /// 1.8.
  #[inline(always)]
  pub fn fixed_x(&self) -> i32 {
    (self.x * 32.0) as i32
  }
  /// Returns the Y value of the position, as a fixed precision float. This is
  /// the Y position multiplied by 32. It is how position packets are sent on
  /// 1.8.
  #[inline(always)]
  pub fn fixed_y(&self) -> i32 {
    (self.y * 32.0) as i32
  }
  /// Returns the Z value of the position, as a fixed precision float. This is
  /// the Z position multiplied by 32. It is how position packets are sent on
  /// 1.8.
  #[inline(always)]
  pub fn fixed_z(&self) -> i32 {
    (self.z * 32.0) as i32
  }
  /// Returns the block that this position is in.
  #[inline(always)]
  pub fn block(&self) -> Pos {
    Pos::new(self.x.floor() as i32, self.y.floor() as i32, self.z.floor() as i32)
  }
  /// Returns the chunk that this position is in. This is the same as
  /// `self.block().chunk()`.
  #[inline(always)]
  pub fn chunk(&self) -> ChunkPos {
    self.block().chunk()
  }
  /// Creates a new error from this position. This should be used to signify
  /// that an invalid position was passed somewhere.
  pub fn err(&self, msg: String) -> FPosError {
    FPosError { pos: *self, msg }
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

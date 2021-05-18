use super::ChunkPos;
use std::{
  error::Error,
  fmt,
  ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Debug)]
pub struct PosError {
  pos: Pos,
  msg: String,
}

impl fmt::Display for PosError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid position: {} {}", self.pos, self.msg)
  }
}

impl Error for PosError {}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pos {
  x: i32,
  y: i32,
  z: i32,
}

impl fmt::Display for Pos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Pos({} {} {})", self.x, self.y, self.z)
  }
}

impl Pos {
  /// Creates a new block position. This can be used to find chunk coordinates,
  /// place blocks, or send a position in a packet.
  pub fn new(x: i32, y: i32, z: i32) -> Self {
    Pos { x, y, z }
  }
  /// Converts a block position from a u64 into a Pos. This format of u64 is
  /// used for versions 1.14 and up. This is also the format used with grpc.
  pub fn from_u64(v: u64) -> Self {
    // Rust will carry the negative sign for signed ints on right shift. So, we need
    // to cast to i64 sometimes.
    let x = (v as i64 >> 38) as i32;
    let y = ((v << 52) as i64 >> 52) as i32;
    let z = ((v << 26) as i64 >> 38) as i32;
    Pos::new(x, y, z)
  }
  /// Converts a block position from a u64 into a Pos. This format of u64 is
  /// used for versions 1.13 and below. This should never be used with grpc.
  pub fn from_old_u64(v: u64) -> Self {
    let x = (v as i64 >> 38) as i32;
    let y = ((v << 26) as i64 >> 52) as i32;
    let z = ((v << 38) as i64 >> 38) as i32;
    Pos::new(x, y, z)
  }
  /// Converts the block position into a u64. This is what should be sent to
  /// clients with versions 1.14 and up. This is also what should be used to
  /// encode a position within a grpc packet.
  pub fn to_u64(self) -> u64 {
    let x = self.x as u64;
    let y = self.y as u64;
    let z = self.z as u64;
    ((x & 0x3ffffff) << 38) | ((z & 0x3ffffff) << 12) | (y & 0xfff)
  }
  /// Converts a block position to a u64. This is what should be used for
  /// clients running 1.13 or below. This should never be used in a grpc
  /// connection.
  pub fn to_old_u64(self) -> u64 {
    let x = self.x as u64;
    let y = self.y as u64;
    let z = self.z as u64;
    ((x & 0x3ffffff) << 38) | ((y & 0xfff) << 26) | (z & 0x3ffffff)
  }
  /// Returns the X value of the position.
  #[inline(always)]
  pub fn x(&self) -> i32 {
    self.x
  }
  /// Returns the Y value of the position.
  #[inline(always)]
  pub fn y(&self) -> i32 {
    self.y
  }
  /// Returns the Z value of the position.
  #[inline(always)]
  pub fn z(&self) -> i32 {
    self.z
  }
  /// Returns the chunk that this block position is in.
  #[inline(always)]
  pub fn chunk(&self) -> ChunkPos {
    ChunkPos::new(self.chunk_x(), self.chunk_z())
  }
  /// Returns this position within the 0, 0 chunk column. That is, the X and Z
  /// are both set to the chunk relative position. The Y value is unchanged.
  #[inline(always)]
  pub fn chunk_rel(&self) -> Pos {
    Pos { x: self.chunk_rel_x(), y: self.y, z: self.chunk_rel_z() }
  }
  /// Returns the block X coordinate within 0..16. This is not the same as X %
  /// 16, because that will give negative numbers for negative X values.
  #[inline(always)]
  pub fn chunk_rel_x(&self) -> i32 {
    (self.x % 16 + 16) % 16
  }
  /// Returns the block Y coordinate within 0..16. This is not the same as Y %
  /// 16, because that will give negative numbers for negative Y values.
  #[inline(always)]
  pub fn chunk_rel_y(&self) -> i32 {
    (self.y % 16 + 16) % 16
  }
  /// Returns the block Z coordinate within 0..16. This is not the same as Z %
  /// 16, because that will give negative numbers for negative Z values.
  #[inline(always)]
  pub fn chunk_rel_z(&self) -> i32 {
    (self.z % 16 + 16) % 16
  }
  /// Returns the chunk X of this position. This is X / 16, rounded to negative
  /// infinity. Rust rounds to zero be default, so this is not the same as X /
  /// 16.
  #[inline(always)]
  pub fn chunk_x(&self) -> i32 {
    if self.x < 0 {
      (self.x + 1) / 16 - 1
    } else {
      self.x / 16
    }
  }
  /// Returns the chunk Y of this position. This is Y / 16, rounded to negative
  /// infinity. Rust rounds to zero be default, so this is not the same as Y /
  /// 16.
  #[inline(always)]
  pub fn chunk_y(&self) -> i32 {
    if self.y < 0 {
      (self.y + 1) / 16 - 1
    } else {
      self.y / 16
    }
  }
  /// Returns the chunk Z of this position. This is Z / 16, rounded to negative
  /// infinity. Rust rounds to zero be default, so this is not the same as Z /
  /// 16.
  #[inline(always)]
  pub fn chunk_z(&self) -> i32 {
    if self.z < 0 {
      (self.z + 1) / 16 - 1
    } else {
      self.z / 16
    }
  }
  /// Creates a new error from this position. This should be used to signify
  /// that an invalid position was passed somewhere.
  pub fn err(&self, msg: String) -> PosError {
    PosError { pos: *self, msg }
  }
}

impl Add for Pos {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
  }
}

impl AddAssign for Pos {
  fn add_assign(&mut self, other: Self) {
    self.x += other.x;
    self.y += other.y;
    self.z += other.z;
  }
}

impl Sub for Pos {
  type Output = Self;
  fn sub(self, other: Self) -> Self {
    Self { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
  }
}

impl SubAssign for Pos {
  fn sub_assign(&mut self, other: Self) {
    self.x -= other.x;
    self.y -= other.y;
    self.z -= other.z;
  }
}

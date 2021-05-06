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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
  /// Converts a block position from a u64 into a Pos struct. This format of u64
  /// is used for versions 1.14 and up. This is also the format used with grpc.
  pub fn from_u64(v: u64) -> Self {
    Pos { x: (v >> 38) as i32, y: (v & 0xfff) as i32, z: (v << 26 >> 38) as i32 }
  }
  /// Converts a block position from a u64 into a Pos struct. This format of u64
  /// is used for versions 1.13 and below. This should never be used with grpc.
  pub fn from_old_u64(v: u64) -> Self {
    Pos { x: (v >> 38) as i32, y: ((v >> 26) & 0xfff) as i32, z: (v & 0x3ffffff) as i32 }
  }
  /// Converts the block position into a u64. This is what should be sent to
  /// clients with versions 1.14 and up. This is also what should be used to
  /// encode a position within a grpc packet.
  pub fn to_u64(&self) -> u64 {
    let x = self.x as u64;
    let y = self.y as u64;
    let z = self.z as u64;
    ((x & 0x3ffffff) << 38) | ((z & 0x3ffffff) << 12) | (y & 0xfff)
  }
  /// Converts a block position to a u64. This is what should be used for
  /// clients running 1.13 or below. This should never be used in a grpc
  /// connection.
  pub fn to_old_u64(&self) -> u64 {
    let x = self.x as u64;
    let y = self.y as u64;
    let z = self.z as u64;
    ((x & 0x3ffffff) << 38) | ((y & 0xfff) << 26) | (z & 0x3ffffff)
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

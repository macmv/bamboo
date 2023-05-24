use bb_macros::Transfer;
use std::{
  fmt,
  ops::{Add, AddAssign, Sub, SubAssign},
};

use super::{Pos, PosIter};

#[derive(Transfer, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ChunkPos {
  x: i32,
  z: i32,
}

impl Default for ChunkPos {
  fn default() -> Self { ChunkPos::new(0, 0) }
}

impl fmt::Display for ChunkPos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "ChunkPos({} {})", self.x, self.z)
  }
}

impl ChunkPos {
  /// Creates a new block position. This can be used to find chunk coordinates,
  /// place blocks, or send a position in a packet.
  pub const fn new(x: i32, z: i32) -> Self { ChunkPos { x, z } }
  /// Returns the X value of the position.
  #[inline(always)]
  pub const fn x(&self) -> i32 { self.x }
  /// Returns the Z value of the position.
  #[inline(always)]
  pub const fn z(&self) -> i32 { self.z }
  /// Returns the minimum block X value of the position. This is just x * 16.
  #[inline(always)]
  pub const fn block_x(&self) -> i32 { self.x * 16 }
  /// Returns the minimum block Z value of the position. This is just x * 16.
  #[inline(always)]
  pub const fn block_z(&self) -> i32 { self.z * 16 }
  /// Returns the minimum block X and Z values of this position. Y will be 0.
  #[inline(always)]
  pub const fn block(&self) -> Pos { Pos::new(self.block_x(), 0, self.block_z()) }

  /// Returns self, with x set to the given value.
  #[inline(always)]
  #[must_use = "with_x returns a modified version of self"]
  pub fn with_x(mut self, x: i32) -> Self {
    self.x = x;
    self
  }
  /// Returns self, with z set to the given value.
  #[inline(always)]
  #[must_use = "with_z returns a modified version of self"]
  pub fn with_z(mut self, z: i32) -> Self {
    self.z = z;
    self
  }
  /// Returns self, with x set to self.x plus the given value.
  #[inline(always)]
  #[must_use = "add_x returns a modified version of self"]
  pub fn add_x(mut self, x: i32) -> Self {
    self.x += x;
    self
  }
  /// Returns self, with z set to self.z plus the given value.
  #[inline(always)]
  #[must_use = "add_z returns a modified version of self"]
  pub fn add_z(mut self, z: i32) -> Self {
    self.z += z;
    self
  }

  /// Creates an iterator that will return every column in the chunk. The Y
  /// coordinate in the position will always be 0.
  #[inline(always)]
  pub fn columns(&self) -> PosIter { self.block().to(self.block() + Pos::new(15, 0, 15)) }
}

impl Add for ChunkPos {
  type Output = Self;
  fn add(self, other: Self) -> Self { Self { x: self.x + other.x, z: self.z + other.z } }
}

impl AddAssign for ChunkPos {
  fn add_assign(&mut self, other: Self) {
    self.x += other.x;
    self.z += other.z;
  }
}

impl Sub for ChunkPos {
  type Output = Self;
  fn sub(self, other: Self) -> Self { Self { x: self.x - other.x, z: self.z - other.z } }
}

impl SubAssign for ChunkPos {
  fn sub_assign(&mut self, other: Self) {
    self.x -= other.x;
    self.z -= other.z;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn iter() {
    let mut total = 0;
    for (i, p) in ChunkPos::new(2, 3).columns().enumerate() {
      total += 1;
      let x = i % 16;
      let z = i / 16;
      dbg!(p);
      assert_eq!(p, Pos::new(x as i32 + 32, 0, z as i32 + 48));
      if i > 256 {
        panic!("invalid index {i}");
      }
    }
    assert_eq!(total, 256);
    total = 0;
    for (i, p) in ChunkPos::new(-1, -3).columns().enumerate() {
      total += 1;
      let x = i % 16;
      let z = i / 16;
      dbg!(p);
      assert_eq!(p, Pos::new(x as i32 - 16, 0, z as i32 - 48));
      if i > 256 {
        panic!("invalid index {i}");
      }
    }
    assert_eq!(total, 256);
  }
}

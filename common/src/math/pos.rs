use super::{terrain::Point, ChunkPos};
use std::{
  error::Error,
  fmt, mem,
  ops::{Add, AddAssign, Range, Sub, SubAssign},
};

#[derive(Debug, PartialEq)]
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
  /// This creates a "unit" position. Whenever a block is placed, the direction
  /// is sent as a byte. This is a value from 0..6. This function parses that
  /// byte, and generates a position that is something like (1, 0, 0) or (0, -1,
  /// 0). The result is intended to be added to a block position, to offset it
  /// by one block. If the value is outside if 0..6, then (0, 0, 0) is returned.
  pub fn dir_from_byte(v: u8) -> Self {
    match v {
      0 => Pos::new(0, -1, 0),
      1 => Pos::new(0, 1, 0),
      2 => Pos::new(0, 0, -1),
      3 => Pos::new(0, 0, 1),
      4 => Pos::new(-1, 0, 0),
      5 => Pos::new(1, 0, 0),
      _ => Pos::new(0, 0, 0),
    }
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
  /// Returns self, with x set to the given value.
  #[inline(always)]
  #[must_use = "with_x returns a modified version of self"]
  pub fn with_x(mut self, x: i32) -> Self {
    self.x = x;
    self
  }
  /// Returns self, with y set to the given value.
  #[inline(always)]
  #[must_use = "with_y returns a modified version of self"]
  pub fn with_y(mut self, y: i32) -> Self {
    self.y = y;
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
  /// Returns self, with y set to self.x plus the given value.
  #[inline(always)]
  #[must_use = "add_y returns a modified version of self"]
  pub fn add_y(mut self, y: i32) -> Self {
    self.y += y;
    self
  }
  /// Returns self, with z set to self.x plus the given value.
  #[inline(always)]
  #[must_use = "add_z returns a modified version of self"]
  pub fn add_z(mut self, z: i32) -> Self {
    self.z += z;
    self
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

  /// Creates a new iterator from the current position to the other position.
  /// This will iterate through every block within a cube where `self` is the
  /// minimum corner, and `end` is the maximum corner.
  #[inline(always)]
  pub fn to(&self, end: Pos) -> PosIter {
    PosIter::new(*self, end)
  }

  /// Uses the `x` and `z` values of self to create a Point. This is mostly used
  /// in terrain generation.
  pub fn to_point(self) -> Point {
    Point::new(self.x, self.z)
  }

  /// Returns the distance to the other position.
  pub fn dist(&self, other: Pos) -> f64 {
    (((self.x - other.x).pow(2) + (self.y - other.y).pow(2) + (self.z - other.z).pow(2)) as f64)
      .sqrt()
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

pub struct PosIter {
  curr:  Pos,
  start: Pos,
  end:   Pos,
}

impl PosIter {
  /// Creates a new inclusive iterator. This will swap around the values in
  /// start and end so that it will always iterate from least to most on x,
  /// then z, then y.
  ///
  /// This might sound like nonsense at first, but this is the order in which
  /// block data is stored. So internally it makes the most sense to do this,
  /// as we can iterate between positions, and then in turn iterate through
  /// the chunk data in order. It also makes sense to do this externally,
  /// simply because going row by row, then layer by layer usually makes the
  /// most sense.
  #[inline(always)]
  pub fn new(mut start: Pos, mut end: Pos) -> Self {
    if start.x > end.x {
      mem::swap(&mut start.x, &mut end.x);
    }
    if start.y > end.y {
      mem::swap(&mut start.y, &mut end.y);
    }
    if start.z > end.z {
      mem::swap(&mut start.z, &mut end.z);
    }
    PosIter { curr: start, start, end }
  }
}

impl From<Range<Pos>> for PosIter {
  fn from(r: Range<Pos>) -> PosIter {
    PosIter::new(r.start, r.end)
  }
}

impl Iterator for PosIter {
  type Item = Pos;

  fn next(&mut self) -> Option<Pos> {
    if self.curr.y > self.end.y {
      return None;
    }
    let ret = self.curr;
    self.curr.x += 1;
    if self.curr.x > self.end.x {
      self.curr.x = self.start.x;
      self.curr.z += 1;
      if self.curr.z > self.end.z {
        self.curr.z = self.start.z;
        self.curr.y += 1;
      }
    }
    Some(ret)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn iter() {
    let mut total = 0;
    for (i, p) in Pos::new(1, 2, 3).to(Pos::new(1, 2, 3)).enumerate() {
      total += 1;
      match i {
        0 => assert_eq!(p, Pos::new(1, 2, 3)),
        _ => panic!("invalid index {}", i),
      }
    }
    assert_eq!(total, 1);
    total = 0;
    for (i, p) in Pos::new(0, 0, 0).to(Pos::new(1, 1, 1)).enumerate() {
      total += 1;
      match i {
        0 => assert_eq!(p, Pos::new(0, 0, 0)),
        1 => assert_eq!(p, Pos::new(1, 0, 0)),
        2 => assert_eq!(p, Pos::new(0, 0, 1)),
        3 => assert_eq!(p, Pos::new(1, 0, 1)),
        4 => assert_eq!(p, Pos::new(0, 1, 0)),
        5 => assert_eq!(p, Pos::new(1, 1, 0)),
        6 => assert_eq!(p, Pos::new(0, 1, 1)),
        7 => assert_eq!(p, Pos::new(1, 1, 1)),
        _ => panic!("invalid index {}", i),
      }
    }
    total = 0;
    for (i, p) in Pos::new(1, 1, 1).to(Pos::new(0, 0, 0)).enumerate() {
      total += 1;
      match i {
        0 => assert_eq!(p, Pos::new(0, 0, 0)),
        1 => assert_eq!(p, Pos::new(1, 0, 0)),
        2 => assert_eq!(p, Pos::new(0, 0, 1)),
        3 => assert_eq!(p, Pos::new(1, 0, 1)),
        4 => assert_eq!(p, Pos::new(0, 1, 0)),
        5 => assert_eq!(p, Pos::new(1, 1, 0)),
        6 => assert_eq!(p, Pos::new(0, 1, 1)),
        7 => assert_eq!(p, Pos::new(1, 1, 1)),
        _ => panic!("invalid index {}", i),
      }
    }
    assert_eq!(total, 8);
  }

  #[test]
  fn pos_decode() {
    let x = 1234;
    let y = 124;
    let z = 5678;
    let p = Pos::from_old_u64(
      ((x as u64 & 0x3ffffff) << 38) | ((y as u64 & 0xfff) << 26) | (z as u64 & 0x3ffffff),
    );
    assert_eq!(p.x(), x);
    assert_eq!(p.y(), y);
    assert_eq!(p.z(), z);

    let x = -15555;
    let y = -120;
    let z = -105661;
    let p = Pos::from_old_u64(
      ((x as u64 & 0x3ffffff) << 38) | ((y as u64 & 0xfff) << 26) | (z as u64 & 0x3ffffff),
    );
    assert_eq!(p.x(), x);
    assert_eq!(p.y(), y);
    assert_eq!(p.z(), z);

    let x = 1234;
    let y = 124;
    let z = 5678;
    let p = Pos::from_u64(
      ((x as u64 & 0x3ffffff) << 38) | ((z as u64 & 0x3ffffff) << 12) | (y as u64 & 0xfff),
    );
    assert_eq!(p.x(), x);
    assert_eq!(p.y(), y);
    assert_eq!(p.z(), z);

    let x = -15555;
    let y = -120;
    let z = -105661;
    let p = Pos::from_u64(
      ((x as u64 & 0x3ffffff) << 38) | ((z as u64 & 0x3ffffff) << 12) | (y as u64 & 0xfff),
    );
    assert_eq!(p.x(), x);
    assert_eq!(p.y(), y);
    assert_eq!(p.z(), z);
  }
}

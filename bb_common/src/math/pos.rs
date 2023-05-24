use super::{ChunkPos, FPos, RelPos, SectionRelPos};
use crate::util::Face;
use bb_macros::Transfer;
use std::{
  error::Error,
  fmt, mem,
  ops::{Add, AddAssign, Range, Sub, SubAssign},
};

#[derive(Debug, PartialEq)]
pub struct PosError {
  pub pos: Pos,
  pub msg: String,
}

impl fmt::Display for PosError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid position: {} {}", self.pos, self.msg)
  }
}

impl Error for PosError {}

#[derive(Transfer, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

impl Default for Pos {
  fn default() -> Self { Pos::new(0, 0, 0) }
}
impl PartialEq<Pos> for RelPos {
  fn eq(&self, other: &Pos) -> bool { self.as_pos() == *other }
}
impl PartialEq<RelPos> for Pos {
  fn eq(&self, other: &RelPos) -> bool { other.as_pos() == *self }
}

impl fmt::Display for Pos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Pos({} {} {})", self.x, self.y, self.z)
  }
}

impl Pos {
  /// Creates a new block position. This can be used to find chunk coordinates,
  /// place blocks, or send a position in a packet.
  pub const fn new(x: i32, y: i32, z: i32) -> Self { Pos { x, y, z } }
  /// Converts a block position from a u64 into a Pos. This format of u64 is
  /// used for versions 1.14 and up. This is also the format used with grpc.
  pub const fn from_u64(v: u64) -> Self {
    // Rust will carry the negative sign for signed ints on right shift. So, we need
    // to cast to i64 sometimes.
    let x = (v as i64 >> 38) as i32;
    let y = ((v << 52) as i64 >> 52) as i32;
    let z = ((v << 26) as i64 >> 38) as i32;
    Pos::new(x, y, z)
  }
  /// Converts a block position from a u64 into a Pos. This format of u64 is
  /// used for versions 1.13 and below. This should never be used with grpc.
  pub const fn from_old_u64(v: u64) -> Self {
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
  pub const fn dir_from_byte(v: u8) -> Self {
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
  pub const fn to_u64(self) -> u64 {
    let x = self.x as u64;
    let y = self.y as u64;
    let z = self.z as u64;
    ((x & 0x3ffffff) << 38) | ((z & 0x3ffffff) << 12) | (y & 0xfff)
  }
  /// Converts a block position to a u64. This is what should be used for
  /// clients running 1.13 or below. This should never be used in a grpc
  /// connection.
  pub const fn to_old_u64(self) -> u64 {
    let x = self.x as u64;
    let y = self.y as u64;
    let z = self.z as u64;
    ((x & 0x3ffffff) << 38) | ((y & 0xfff) << 26) | (z & 0x3ffffff)
  }
  /// Returns the X value of the position.
  #[inline(always)]
  pub const fn x(&self) -> i32 { self.x }
  /// Returns the Y value of the position.
  #[inline(always)]
  pub const fn y(&self) -> i32 { self.y }
  /// Returns the Z value of the position.
  #[inline(always)]
  pub const fn z(&self) -> i32 { self.z }
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
  /// Returns self, with y set to self.y plus the given value.
  #[inline(always)]
  #[must_use = "add_y returns a modified version of self"]
  pub fn add_y(mut self, y: i32) -> Self {
    self.y += y;
    self
  }
  /// Returns self, with z set to self.z plus the given value.
  #[inline(always)]
  #[must_use = "add_z returns a modified version of self"]
  pub fn add_z(mut self, z: i32) -> Self {
    self.z += z;
    self
  }
  /// Returns the chunk that this block position is in.
  #[inline(always)]
  pub const fn chunk(&self) -> ChunkPos { ChunkPos::new(self.chunk_x(), self.chunk_z()) }
  /// Returns this position within the 0, 0, 0 chunk cube. That is, the X, Y and
  /// Z are all set to their chunk relative position.
  #[inline(always)]
  pub const fn chunk_rel(&self) -> RelPos {
    RelPos::new(self.chunk_rel_x() as u8, self.y, self.chunk_rel_z() as u8)
  }
  /// Returns this position within the 0, 0, 0 chunk section. That is, the X, Y
  /// and Z are all set to their chunk relative position.
  #[inline(always)]
  pub const fn chunk_section_rel(&self) -> SectionRelPos {
    SectionRelPos::new(self.chunk_rel_x() as u8, self.chunk_rel_y() as u8, self.chunk_rel_z() as u8)
  }
  /// Returns the block X coordinate within 0..16. This is not the same as X %
  /// 16, because that will give negative numbers for negative X values.
  #[inline(always)]
  pub const fn chunk_rel_x(&self) -> i32 { (self.x % 16 + 16) % 16 }
  /// Returns the block Y coordinate within 0..16. This is not the same as Y %
  /// 16, because that will give negative numbers for negative Y values.
  #[inline(always)]
  pub const fn chunk_rel_y(&self) -> i32 { (self.y % 16 + 16) % 16 }
  /// Returns the block Z coordinate within 0..16. This is not the same as Z %
  /// 16, because that will give negative numbers for negative Z values.
  #[inline(always)]
  pub const fn chunk_rel_z(&self) -> i32 { (self.z % 16 + 16) % 16 }
  /// Returns the chunk X of this position. This is X / 16, rounded to negative
  /// infinity. Rust rounds to zero be default, so this is not the same as X /
  /// 16.
  #[inline(always)]
  pub const fn chunk_x(&self) -> i32 {
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
  pub const fn chunk_y(&self) -> i32 {
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
  pub const fn chunk_z(&self) -> i32 {
    if self.z < 0 {
      (self.z + 1) / 16 - 1
    } else {
      self.z / 16
    }
  }
  /// Creates a new error from this position. This should be used to signify
  /// that an invalid position was passed somewhere.
  pub fn err(&self, msg: String) -> PosError { PosError { pos: *self, msg } }

  /// Creates a new iterator from the current position to the other position.
  /// This will iterate through every block within a cube where `self` is the
  /// minimum corner, and `end` is the maximum corner.
  #[inline(always)]
  pub fn to(&self, end: Pos) -> PosIter { PosIter::new(*self, end) }

  /// Returns the distance to the other position.
  pub fn dist(&self, other: Pos) -> f64 {
    (((self.x - other.x).pow(2) + (self.y - other.y).pow(2) + (self.z - other.z).pow(2)) as f64)
      .sqrt()
  }
  /// Returns the squared distance to the other position. Since block positions
  /// are always ints, this will also always be exactly an int.
  pub const fn dist_squared(&self, other: Pos) -> i32 {
    (self.x - other.x).pow(2) + (self.y - other.y).pow(2) + (self.z - other.z).pow(2)
  }

  /// Returns the min of each element in self and other.
  ///
  /// This doesn't use [`std::cmp::Ord`], as two positions aren't really
  /// "larger" or "smaller" than one another. Instead, this just acts on `X`,
  /// `Y`, and `Z` independently.
  ///
  /// # Example
  ///
  /// ```
  /// # use bb_common::math::Pos;
  /// // Basic usage
  /// assert_eq!(Pos::new(1, 5, 6).min(Pos::new(3, 3, 3)), Pos::new(1, 3, 3));
  ///
  /// // Keep a position within a chunk relative range (this would be useful for
  /// // filling a cube, for example).
  /// let pos = Pos::new(2, 34, 14);
  /// let new_pos = pos + Pos::new(3, 3, 3);
  /// // `new_pos` is be outside a chunk, and might break something
  /// assert_eq!(new_pos, Pos::new(5, 37, 17));
  /// // This would clamp the `new_pos` within the chunk boundaries
  /// assert_eq!(new_pos.min(Pos::new(15, 255, 15)), Pos::new(5, 37, 15));
  /// ```
  pub fn min(&self, other: Pos) -> Pos {
    Pos::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z))
  }

  /// Returns the max of each element in self and other.
  ///
  /// This doesn't use [`std::cmp::Ord`], as two positions aren't really
  /// "larger" or "smaller" than one another. Instead, this just acts on `X`,
  /// `Y`, and `Z` independently.
  ///
  /// # Example
  ///
  /// ```
  /// # use bb_common::math::Pos;
  /// // Basic usage
  /// assert_eq!(Pos::new(1, 5, 6).max(Pos::new(3, 3, 3)), Pos::new(3, 5, 6));
  ///
  /// // Keep a position within a chunk relative range (this would be useful for
  /// // filling a cube, for example).
  /// let pos = Pos::new(2, 34, 14);
  /// let new_pos = pos - Pos::new(3, 3, 3);
  /// // `new_pos` is be outside a chunk, and might break something
  /// assert_eq!(new_pos, Pos::new(-1, 31, 11));
  /// // This would clamp the `new_pos` within the chunk boundries
  /// assert_eq!(new_pos.max(Pos::new(0, 0, 0)), Pos::new(0, 31, 11));
  /// ```
  pub fn max(&self, other: Pos) -> Pos {
    Pos::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z))
  }

  /// Returns the minimum and maximum of each value of the three positions. The
  /// first argument returned is the min, and the second argument returned is
  /// the max.
  ///
  /// # Example
  ///
  /// ```
  /// # use bb_common::math::Pos;
  /// assert_eq!(
  ///   Pos::new(1, 5, 6).min_max(Pos::new(3, 3, 3)),
  ///   (Pos::new(1, 3, 3), Pos::new(3, 5, 6))
  /// );
  /// // different syntax, does the same thing
  /// assert_eq!(
  ///   Pos::min_max(Pos::new(1, 5, 6), Pos::new(3, 3, 3)),
  ///   (Pos::new(1, 3, 3), Pos::new(3, 5, 6))
  /// );
  /// ```
  pub fn min_max(self, other: Pos) -> (Pos, Pos) {
    (
      Pos::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z)),
      Pos::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z)),
    )
  }

  /// Converts this position to a floating point position, with the X and Z
  /// offset by 0.5. This is very common when spawning entities, as generally,
  /// it is convenient to put them in the center of a block.
  #[inline]
  pub fn center(&self) -> FPos {
    FPos::new(self.x as f64 + 0.5, self.y as f64, self.z as f64 + 0.5)
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

impl Add<Face> for Pos {
  type Output = Self;
  fn add(self, other: Face) -> Self { self + other.as_dir() }
}
impl AddAssign<Face> for Pos {
  fn add_assign(&mut self, other: Face) { *self += other.as_dir() }
}

impl Sub<Face> for Pos {
  type Output = Self;
  fn sub(self, other: Face) -> Self { self - other.as_dir() }
}
impl SubAssign<Face> for Pos {
  fn sub_assign(&mut self, other: Face) { *self -= other.as_dir() }
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

  /// Returns true if the given position is within the iterator. Like the
  /// iterator, this is inclusive for the minimum and maximum.
  pub fn contains(&self, pos: Pos) -> bool {
    pos.x >= self.start.x
      && pos.y >= self.start.y
      && pos.z >= self.start.z
      && pos.x <= self.end.x
      && pos.y <= self.end.y
      && pos.z <= self.end.z
  }
}

impl From<Range<Pos>> for PosIter {
  fn from(r: Range<Pos>) -> PosIter { PosIter::new(r.start, r.end) }
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

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = ((self.end.x - self.start.x + 1)
      * (self.end.y - self.start.y + 1)
      * (self.end.z - self.start.z + 1)) as usize;
    (len, Some(len))
  }
}

impl ExactSizeIterator for PosIter {
  fn len(&self) -> usize {
    ((self.end.x - self.start.x + 1)
      * (self.end.y - self.start.y + 1)
      * (self.end.z - self.start.z + 1)) as usize
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
        _ => panic!("invalid index {i}"),
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
        _ => panic!("invalid index {i}"),
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
        _ => panic!("invalid index {i}"),
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

use super::{Face, Pos, PosError};
use std::{
  fmt, mem,
  ops::{Add, AddAssign, Range},
};

/// A position relative to a chunk cube. This has X, Y and Z set to only be in
/// the range `0..16`.
///
/// # Safety
///
/// The X, Y, and Z values will never be outside 0..16. This can be relied on
/// within unsafe code.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SectionRelPos {
  x: u8,
  y: u8,
  z: u8,
}

impl fmt::Display for SectionRelPos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "RelPos({} {} {})", self.x, self.y, self.z)
  }
}

/// A position relative to a chunk column. This has X and Z set to only be in
/// the range `0..16`.
///
/// # Safety
///
/// The X and Z values will never be outside 0..16. This can be relied on
/// within unsafe code.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RelPos {
  x: u8,
  y: i32,
  z: u8,
}

impl fmt::Display for RelPos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "RelPos({} {} {})", self.x, self.y, self.z)
  }
}

impl SectionRelPos {
  /// A chunk-relative position.
  ///
  /// # Panics
  /// If the X, Y, or Z is greater than 15.
  pub fn new(x: u8, y: u8, z: u8) -> Self {
    if x >= 16 || y >= 16 || z >= 16 {
      panic!("X, Y and Z must be within 0..16");
    }
    SectionRelPos { x, y, z }
  }
  /// Returns the X position. This won't return a value above 15.
  #[inline(always)]
  pub fn x(&self) -> u8 { self.x }
  /// Returns the Y position.
  #[inline(always)]
  pub fn y(&self) -> u8 { self.y }
  /// Returns the Z position. This won't return a value above 15.
  #[inline(always)]
  pub fn z(&self) -> u8 { self.z }

  /// Creates a new error from this position. This should be used to signify
  /// that an invalid position was passed somewhere.
  pub fn err(&self, msg: String) -> PosError { PosError { pos: self.as_pos(), msg } }

  /// Returns this relative position as an absolute position. This will be in
  /// the `0,0,0` chunk cube.
  pub fn as_pos(&self) -> Pos { Pos::new(self.x.into(), self.y.into(), self.z.into()) }

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
  pub fn min_max(self, other: SectionRelPos) -> (SectionRelPos, SectionRelPos) {
    (
      SectionRelPos::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z)),
      SectionRelPos::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z)),
    )
  }
}

impl RelPos {
  /// A chunk column relative position.
  ///
  /// # Panics
  /// If the X, or Z is greater than 15.
  pub fn new(x: u8, y: i32, z: u8) -> Self {
    if x >= 16 || z >= 16 {
      panic!("X and Z must be within 0..16");
    }
    RelPos { x, y, z }
  }
  pub fn new_opt(x: u8, y: i32, z: u8) -> Option<Self> {
    if x >= 16 || z >= 16 {
      None
    } else {
      Some(RelPos { x, y, z })
    }
  }
  /// Returns the X position. This won't return a value above 15.
  #[inline(always)]
  pub fn x(&self) -> u8 { self.x }
  /// Returns the Y position.
  #[inline(always)]
  pub fn y(&self) -> i32 { self.y }
  /// Returns the Z position. This won't return a value above 15.
  #[inline(always)]
  pub fn z(&self) -> u8 { self.z }

  /// Returns self, with x set to self.x plus the given value.
  #[inline(always)]
  #[must_use = "add_x returns a modified version of self"]
  pub fn add_x(mut self, x: u8) -> Self {
    if self.x.checked_add(x).unwrap_or(255) > 16 {
      panic!("cannot add X with overflow: {} + {}", self.x, x);
    }
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
  pub fn add_z(mut self, z: u8) -> Self {
    if self.z.checked_add(z).unwrap_or(255) > 16 {
      panic!("cannot add Z with overflow: {} + {}", self.z, z);
    }
    self.z += z;
    self
  }

  /// Returns self, with y set to the given value.
  #[inline(always)]
  #[must_use = "with_y returns a modified version of self"]
  pub fn with_y(mut self, y: i32) -> Self {
    self.y = y;
    self
  }

  /// Returns the block Y coordinate within 0..16. This is not the same as Y %
  /// 16, because that will give negative numbers for negative Y values.
  #[inline(always)]
  pub fn chunk_rel_y(&self) -> i32 { (self.y % 16 + 16) % 16 }

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
  /// Returns this position within the 0, 0, 0 chunk section. That is, the X, Y
  /// and Z are all set to their chunk relative position.
  #[inline(always)]
  pub fn section_rel(&self) -> SectionRelPos {
    SectionRelPos::new(self.x, self.chunk_rel_y() as u8, self.z)
  }

  /// Creates a new error from this position. This should be used to signify
  /// that an invalid position was passed somewhere.
  pub fn err(&self, msg: String) -> PosError { PosError { pos: self.as_pos(), msg } }

  /// Returns this relative position as an absolute position. This will be in
  /// the `0,0` chunk column.
  pub fn as_pos(&self) -> Pos { Pos::new(self.x.into(), self.y, self.z.into()) }

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
  pub fn min_max(self, other: RelPos) -> (RelPos, RelPos) {
    (
      RelPos::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z)),
      RelPos::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z)),
    )
  }

  pub fn checked_add(&self, face: Face) -> Option<Self> {
    let (x, y, z): (i8, _, i8) = match face {
      Face::East => (-1, 0, 0),
      Face::West => (1, 0, 0),
      Face::Up => (0, 1, 0),
      Face::Down => (0, -1, 0),
      Face::South => (0, 0, 1),
      Face::North => (0, 0, -1),
    };
    RelPos::new_opt(
      if x >= 0 {
        u8::checked_add(self.x, x as u8)?
      } else {
        u8::checked_sub(self.x, x.unsigned_abs())?
      },
      self.y.checked_add(y)?,
      if z >= 0 {
        u8::checked_add(self.z, z as u8)?
      } else {
        u8::checked_sub(self.z, z.unsigned_abs())?
      },
    )
  }

  /// Creates a new iterator from the current position to the other position.
  /// This will iterate through every block within a cube where `self` is the
  /// minimum corner, and `end` is the maximum corner.
  #[inline(always)]
  pub fn to(&self, end: RelPos) -> RelPosIter { RelPosIter::new(*self, end) }
}

impl Add for RelPos {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    if self.x.checked_add(other.x).unwrap_or(255) > 16
      || self.z.checked_add(other.z).unwrap_or(255) > 16
    {
      panic!("cannot add with overflow: {self} + {other}");
    }
    Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
  }
}

impl AddAssign for RelPos {
  fn add_assign(&mut self, other: Self) {
    if self.x.checked_add(other.x).unwrap_or(255) > 16
      || self.z.checked_add(other.z).unwrap_or(255) > 16
    {
      panic!("cannot add with overflow: {self} + {other}");
    }
    self.x += other.x;
    self.y += other.y;
    self.z += other.z;
  }
}

pub struct RelPosIter {
  curr:  RelPos,
  start: RelPos,
  end:   RelPos,
}

impl RelPosIter {
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
  pub fn new(mut start: RelPos, mut end: RelPos) -> Self {
    if start.x > end.x {
      mem::swap(&mut start.x, &mut end.x);
    }
    if start.y > end.y {
      mem::swap(&mut start.y, &mut end.y);
    }
    if start.z > end.z {
      mem::swap(&mut start.z, &mut end.z);
    }
    RelPosIter { curr: start, start, end }
  }
}

impl From<Range<RelPos>> for RelPosIter {
  fn from(r: Range<RelPos>) -> RelPosIter { RelPosIter::new(r.start, r.end) }
}

impl Iterator for RelPosIter {
  type Item = RelPos;

  fn next(&mut self) -> Option<RelPos> {
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
    let len = (self.end.x - self.start.x + 1) as usize
      * (self.end.y - self.start.y + 1) as usize
      * (self.end.z - self.start.z + 1) as usize;
    (len, Some(len))
  }
}

impl ExactSizeIterator for RelPosIter {
  fn len(&self) -> usize {
    (self.end.x - self.start.x + 1) as usize
      * (self.end.y - self.start.y + 1) as usize
      * (self.end.z - self.start.z + 1) as usize
  }
}

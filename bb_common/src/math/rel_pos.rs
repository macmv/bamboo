use super::{Face, Pos, PosError};

/// A position relative to a chunk cube. This has X, Y and Z set to only be in
/// the range `0..16`.
///
/// # Safety
///
/// The X, Y, and Z values will never be outside 0..16. This can be relied on
/// within unsafe code.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RelPos {
  x: u8,
  y: u8,
  z: u8,
}
/// A position relative to a chunk column. This has X and Z set to only be in
/// the range `0..16`.
///
/// # Safety
///
/// The X and Z values will never be outside 0..16. This can be relied on
/// within unsafe code.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ColRelPos {
  x: u8,
  y: i32,
  z: u8,
}

impl RelPos {
  /// A chunk-relative position.
  ///
  /// # Panics
  /// If the X, Y, or Z is greater than 15.
  pub fn new(x: u8, y: u8, z: u8) -> Self {
    if x >= 16 || y >= 16 || z >= 16 {
      panic!("X, Y and Z must be within 0..16");
    }
    RelPos { x, y, z }
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
  pub fn min_max(self, other: RelPos) -> (RelPos, RelPos) {
    (
      RelPos::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z)),
      RelPos::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z)),
    )
  }
}

impl ColRelPos {
  /// A chunk column relative position.
  ///
  /// # Panics
  /// If the X, or Z is greater than 15.
  pub fn new(x: u8, y: i32, z: u8) -> Self {
    if x >= 16 || z >= 16 {
      panic!("X and Z must be within 0..16");
    }
    ColRelPos { x, y, z }
  }
  pub fn new_opt(x: u8, y: i32, z: u8) -> Option<Self> {
    if x >= 16 || z >= 16 {
      None
    } else {
      Some(ColRelPos { x, y, z })
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
  /// Returns this position within the 0, 0, 0 chunk cube. That is, the X, Y and
  /// Z are all set to their chunk relative position.
  #[inline(always)]
  pub fn chunk_rel(&self) -> RelPos { RelPos::new(self.x, self.chunk_rel_y() as u8, self.z) }

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
  pub fn min_max(self, other: ColRelPos) -> (ColRelPos, ColRelPos) {
    (
      ColRelPos::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z)),
      ColRelPos::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z)),
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
    ColRelPos::new_opt(
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
}

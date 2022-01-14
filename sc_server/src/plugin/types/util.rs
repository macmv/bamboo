use super::{add_from, wrap};
use sc_common::math::{FPos, Pos};
use sugarlang::define_ty;

wrap!(Pos, SlPos);
wrap!(FPos, SlFPos);

/// A block position. This stores X, Y, and Z coordinates.
///
/// If you need a player position, use `FPos` instead.
#[define_ty(path = "sugarcane::util::Pos")]
impl SlPos {
  /// Creates a new block position, with the given X, Y, and Z coordinates.
  pub fn new(x: i32, y: i32, z: i32) -> Self { SlPos { inner: Pos::new(x, y, z) } }
  /// Returns the X position of this block.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.x() // returns 5
  /// ```
  pub fn x(&self) -> i32 { self.inner.x() }
  /// Returns the Y position of this block.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.y() // returns 6
  /// ```
  pub fn y(&self) -> i32 { self.inner.y() }
  /// Returns the Z position of this block.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.z() // returns 7
  /// ```
  pub fn z(&self) -> i32 { self.inner.z() }
}

/// An entity position. This stores X, Y, and Z coordinates as floats.
///
/// If you need a block position, use `Pos` instead.
#[define_ty(path = "sugarcane::util::FPos")]
impl SlFPos {
  /// Returns the X position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.x() // returns 5.5
  /// ```
  pub fn x(&self) -> f64 { self.inner.x() }
  /// Returns the Y position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.y() // returns 6.0
  /// ```
  pub fn y(&self) -> f64 { self.inner.y() }
  /// Returns the Z position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.z() // returns 7.2
  /// ```
  pub fn z(&self) -> f64 { self.inner.z() }
}

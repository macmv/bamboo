use super::{add_from, wrap, wrap_eq};
use bb_common::{
  math::{ChunkPos, FPos, Pos},
  util::UUID,
};
use bb_server_macros::define_ty;

wrap_eq!(Pos, PPos);
wrap_eq!(ChunkPos, PChunkPos);
wrap!(FPos, PFPos);
wrap_eq!(UUID, PUUID);

/// A block position. This stores X, Y, and Z coordinates as ints.
///
/// If you need a player position, use `FPos` (for float position) instead.
#[define_ty(panda_path = "bamboo::util::Pos", panda_map_key = true)]
impl PPos {
  /// Creates a new block position, with the given X, Y, and Z coordinates.
  pub fn new(x: i32, y: i32, z: i32) -> Self { PPos { inner: Pos::new(x, y, z) } }
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

/// A chunk position. This stores X and Z coordinates.
///
/// If you need a block position, use `Pos` instead.
#[define_ty(panda_path = "bamboo::util::ChunkPos", panda_map_key = true)]
impl PChunkPos {
  /// Creates a new chunk position, with the given X and Z coordinates.
  pub fn new(x: i32, z: i32) -> Self { PChunkPos { inner: ChunkPos::new(x, z) } }
  /// Returns the X position of this chuk.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.x() // returns 5
  /// ```
  pub fn x(&self) -> i32 { self.inner.x() }
  /// Returns the Z position of this chuk.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.z() // returns 7
  /// ```
  pub fn z(&self) -> i32 { self.inner.z() }
}

/// An entity position. This stores X, Y, and Z coordinates as floats.
///
/// If you need a block position, use `Pos` instead.
#[define_ty(panda_path = "bamboo::util::FPos")]
impl PFPos {
  /// Creates a new floating point position, with the given X, Y, and Z
  /// coordinates.
  pub fn new(x: f64, y: f64, z: f64) -> Self { PFPos { inner: FPos::new(x, y, z) } }
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

/// A UUID. This is used as a unique identifier for players and entities.
#[define_ty(panda_path = "bamboo::util::UUID", panda_map_key = true)]
impl PUUID {
  /// Returns the UUID as a string, with dashes inserted.
  pub fn to_s(&self) -> String { self.inner.as_dashed_str() }
}

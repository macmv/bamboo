use super::{add_from, wrap, wrap_eq};
use crate::math::Vec3;
use bb_common::{
  math::{ChunkPos, FPos, Pos},
  util::UUID,
};
use bb_server_macros::define_ty;

wrap_eq!(Pos, PPos);
wrap_eq!(ChunkPos, PChunkPos);
wrap!(FPos, PFPos);
wrap!(Vec3, PVec3);
wrap_eq!(UUID, PUUID);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "python_plugins", ::pyo3::pyclass)]
pub struct PDuration {
  pub ticks: u32,
}

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

  /// Returns a new position, with the X set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.with_x(10) // returns Pos::new(10, 6, 7)
  /// ```
  pub fn with_x(&self, new_x: i32) -> Self { self.inner.with_x(new_x).into() }
  /// Returns a new position, with the Y set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.with_y(10) // returns Pos::new(5, 10, 7)
  /// ```
  pub fn with_y(&self, new_y: i32) -> Self { self.inner.with_y(new_y).into() }
  /// Returns a new position, with the Z set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.with_z(10) // returns Pos::new(5, 6, 10)
  /// ```
  pub fn with_z(&self, new_z: i32) -> Self { self.inner.with_z(new_z).into() }

  /// Adds the given number in the X axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.add_x(10) // returns Pos::new(15, 6, 7)
  /// ```
  pub fn add_x(&self, offset: i32) -> Self { self.inner.add_x(offset).into() }
  /// Adds the given number in the Y axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.add_y(10) // returns Pos::new(5, 16, 7)
  /// ```
  pub fn add_y(&self, offset: i32) -> Self { self.inner.add_y(offset).into() }
  /// Adds the given number in the Z axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.add_z(10) // returns Pos::new(5, 6, 17)
  /// ```
  pub fn add_z(&self, offset: i32) -> Self { self.inner.add_z(offset).into() }
}

/// A chunk position. This stores X and Z coordinates.
///
/// If you need a block position, use `Pos` instead.
#[define_ty(panda_path = "bamboo::util::ChunkPos", panda_map_key = true)]
impl PChunkPos {
  /// Creates a new chunk position, with the given X and Z coordinates.
  pub fn new(x: i32, z: i32) -> Self { PChunkPos { inner: ChunkPos::new(x, z) } }
  /// Returns the X position of this chunk.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.x() // returns 5
  /// ```
  pub fn x(&self) -> i32 { self.inner.x() }
  /// Returns the Z position of this chunk.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.z() // returns 7
  /// ```
  pub fn z(&self) -> i32 { self.inner.z() }

  /// Returns a new position, with the X set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.with_x(10) // returns ChunkPos::new(10, 7)
  /// ```
  pub fn with_x(&self, new_x: i32) -> Self { self.inner.with_x(new_x).into() }
  /// Returns a new position, with the Z set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.with_z(10) // returns ChunkPos::new(5, 10)
  /// ```
  pub fn with_z(&self, new_z: i32) -> Self { self.inner.with_z(new_z).into() }

  /// Adds the given number in the X axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.add_x(10) // returns ChunkPos::new(15, 7)
  /// ```
  pub fn add_x(&self, offset: i32) -> Self { self.inner.add_x(offset).into() }
  /// Adds the given number in the Z axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.add_z(10) // returns ChunkPos::new(5, 17)
  /// ```
  pub fn add_z(&self, offset: i32) -> Self { self.inner.add_z(offset).into() }
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

  /// Returns a new position, with the X set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.with_x(10.0) // returns FPos::new(10.0, 6.0, 7.2)
  /// ```
  pub fn with_x(&self, new_x: f64) -> Self { self.inner.with_x(new_x).into() }
  /// Returns a new position, with the Y set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.with_y(10.0) // returns FPos::new(5.5, 10.0, 7.2)
  /// ```
  pub fn with_y(&self, new_y: f64) -> Self { self.inner.with_y(new_y).into() }
  /// Returns a new position, with the Z set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.with_z(10.0) // returns FPos::new(5.5, 6.0, 10.0)
  /// ```
  pub fn with_z(&self, new_z: f64) -> Self { self.inner.with_z(new_z).into() }

  /// Adds the given number in the X axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.add_x(100.0) // returns FPos::new(105.5, 6.0, 7.2)
  /// ```
  pub fn add_x(&self, offset: f64) -> Self { self.inner.add_x(offset).into() }
  /// Adds the given number in the Y axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.add_y(100.0) // returns FPos::new(5.5, 106.0, 7.2)
  /// ```
  pub fn add_y(&self, offset: f64) -> Self { self.inner.add_y(offset).into() }
  /// Adds the given number in the Z axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.add_z(100.0) // returns FPos::new(5.5, 6.0, 107.2)
  /// ```
  pub fn add_z(&self, offset: f64) -> Self { self.inner.add_z(offset).into() }

  /// Returns the block that this position is in. This will round all 3 axis
  /// down.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.block() // returns Pos::new(5, 6, 7)
  /// ```
  pub fn block(&self) -> PPos { self.inner.block().into() }
}

/// A vector. This stores X, Y, and Z coordinates as floats.
///
/// If you need a position in the world, use `FPos` instead. This is used for
/// entity velocities, and raycasting math.
#[define_ty(panda_path = "bamboo::util::Vec3")]
impl PVec3 {
  /// Creates a new floating point position, with the given X, Y, and Z
  /// coordinates.
  pub fn new(x: f64, y: f64, z: f64) -> Self { PVec3 { inner: Vec3::new(x, y, z) } }

  /// Returns the X axis of this vector.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Vec3::new(5.5, 6.0, 7.2)
  /// pos.x // returns 5.5
  /// ```
  #[field]
  pub fn x(&self) -> f64 { self.inner.x }
  /// Returns the Y axis of this vector.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.y() // returns 6.0
  /// ```
  #[field]
  pub fn y(&self) -> f64 { self.inner.y }
  /// Returns the Z axis of this vector.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Vec3::new(5.5, 6.0, 7.2)
  /// pos.z() // returns 7.2
  /// ```
  #[field]
  pub fn z(&self) -> f64 { self.inner.z }
}

/// A UUID. This is used as a unique identifier for players and entities.
#[define_ty(panda_path = "bamboo::util::UUID", panda_map_key = true)]
impl PUUID {
  /// Returns the UUID as a string, with dashes inserted.
  pub fn to_s(&self) -> String { self.inner.as_dashed_str() }
}

/// A duration. This is a number of ticks internally, and can be created from
/// a number of ticks, seconds, or minutes.
#[define_ty(panda_path = "bamboo::util::Duration")]
impl PDuration {
  /// Returns a duration for the number of seconds specified.
  pub fn from_secs(secs: u32) -> Self { PDuration { ticks: secs * 20 } }
  /// Returns a duration for the number of minutes specified.
  pub fn from_minutes(minutes: u32) -> Self { PDuration { ticks: minutes * 20 * 60 } }
}

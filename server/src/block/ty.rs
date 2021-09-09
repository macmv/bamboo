use num_derive::{FromPrimitive, ToPrimitive};
use std::{error::Error, fmt, str::FromStr};

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
#[derive(Debug)]
pub struct Type {
  pub(super) kind:  Kind,
  pub(super) state: u32,
}

impl Type {
  /// Returns the block kind that this state comes from.
  pub fn kind(&self) -> &Kind {
    &self.kind
  }
  /// Gets the block id for the given version. This will always be the latest
  /// blockstate id.
  pub fn id(&self) -> u32 {
    self.state
  }
}

#[derive(Debug)]
pub struct InvalidBlock(String);

impl fmt::Display for InvalidBlock {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid block name: {}", self.0)
  }
}

impl Error for InvalidBlock {}

include!(concat!(env!("OUT_DIR"), "/block/ty.rs"));

impl Kind {
  /// Returns the kind as an u32. This is used in the versioning arrays, and in
  /// plugin code, so that ints can be passed around instead of enums.
  pub fn id(self) -> u32 {
    num::ToPrimitive::to_u32(&self).unwrap()
  }
  /// Converts the given number to a block kind. If the number is invalid, this
  /// returns Kind::Air.
  pub fn from_u32(id: u32) -> Self {
    num::FromPrimitive::from_u32(id).unwrap_or(Kind::Air)
  }
}

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  state:            u32,
  // A list of types in order. This will always be at least one element long.
  pub(super) types: &'static [Type],
  // The default type. This is an index into types.
  default_index:    u32,
}

impl Data {
  /// Returns the default type for this kind. This is usually what should be
  /// placed down when the user right clicks on a block. Sometimes, for blocks
  /// like stairs or doors, the type that should be placed must be computed when
  /// they place the block, as things like their position/rotation affect which
  /// block gets placed.
  pub fn default_type(&self) -> &Type {
    &self.types[self.default_index as usize]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate() {
    dbg!(generate_kinds());
    // Used to show debug output.
    // assert!(false);
  }
}

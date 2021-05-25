use num_derive::{FromPrimitive, ToPrimitive};

// Generated from the latest version of minecraft's output. See build.rs for
// more.
include!(concat!(env!("OUT_DIR"), "/entity/type.rs"));

impl Type {
  /// Returns the kind as a u32. Should only be used to index into the
  /// converter's internal table of block kinds.
  pub fn to_u32(self) -> u32 {
    num::ToPrimitive::to_u32(&self).unwrap()
  }
  /// Returns the item with the given id. If the id is invalid, this returns
  /// `Type::Air`.
  pub fn from_u32(v: u32) -> Type {
    num::FromPrimitive::from_u32(v).unwrap_or(Type::Air)
  }
}

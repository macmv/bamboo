use data::generate_blocks;
use num_derive::{FromPrimitive, ToPrimitive};
use std::{error::Error, fmt, str::FromStr};

#[derive(Debug)]
pub struct InvalidBlock(String);

impl fmt::Display for InvalidBlock {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid block name: {}", self.0)
  }
}

impl Error for InvalidBlock {}

generate_blocks!();

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

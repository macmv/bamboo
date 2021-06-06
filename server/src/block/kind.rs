use num_derive::ToPrimitive;
use std::{error::Error, fmt, str::FromStr};

#[derive(Debug)]
pub struct InvalidBlock(String);

impl fmt::Display for InvalidBlock {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid block name: {}", self.0)
  }
}

impl Error for InvalidBlock {}

// Generated from the latest version of minecraft's output. See build.rs for
// more.
include!(concat!(env!("OUT_DIR"), "/block/kind.rs"));

impl Kind {
  /// Returns the kind as an i32. Should only be used to index into the
  /// converter's internal table of block kinds.
  pub fn id(self) -> u32 {
    num::ToPrimitive::to_u32(&self).unwrap()
  }
}

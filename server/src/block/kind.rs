use num_derive::ToPrimitive;

// Generated from the latest version of minecraft's output. See build.rs for
// more.
include!(concat!(env!("OUT_DIR"), "/block/kind.rs"));

impl Kind {
  /// Returns the kind as an i32. Should only be used to index into the
  /// converter's internal table of block kinds.
  pub fn to_u32(self) -> u32 {
    num::ToPrimitive::to_u32(&self).unwrap()
  }
}

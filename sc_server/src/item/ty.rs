use crate::block;
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Type {
  Air,
  Snowball,
}

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  display_name:   &'static str,
  stack_size:     u32,
  block_to_place: block::Kind,
}

impl Data {
  pub fn display_name(&self) -> &str {
    &self.display_name
  }

  /// Returns the block to place from this item.
  pub fn block_to_place(&self) -> block::Kind {
    self.block_to_place
  }
}

// Creates the type enum, and the generate_data function
include!(concat!(env!("OUT_DIR"), "/item/ty.rs"));

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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_blocks() {
    let data = generate_items();

    // Sanity check some random blocks
    assert_eq!(data[0].block_to_place, block::Kind::Air);
    assert_eq!(data[1].block_to_place, block::Kind::Stone);
    assert_eq!(data[2].block_to_place, block::Kind::Granite);
    assert_eq!(data[182].block_to_place, block::Kind::DiamondBlock);
    assert_eq!(data[430].block_to_place, block::Kind::Observer);
    // Used to show debug output.
    // assert!(false);
  }
}

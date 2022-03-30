use num::{FromPrimitive, ToPrimitive};
use num_derive::{FromPrimitive, ToPrimitive};
use std::{error::Error, fmt, str::FromStr};

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  ty:   Type,
  name: &'static str,
  id:   u32,
}

impl Data {
  /// Returns the type of this item. This is copyable, and is a unique ID that
  /// can be easily passed around.
  pub fn ty(&self) -> Type { self.ty }
  /// Returns the item's ID. This is the latest protocol ID.
  pub fn id(&self) -> u32 { self.id }
  /// Returns the name of this item. This is something like `minecraft:stone`.
  pub fn name(&self) -> &'static str { self.name }
}

#[derive(Debug)]
pub struct InvalidItem(String);

impl fmt::Display for InvalidItem {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid item name: {}", self.0)
  }
}

impl Error for InvalidItem {}

// Creates the type enum, and the generate_data function
include!(concat!(env!("OUT_DIR"), "/item/ty.rs"));

impl Type {
  /// Returns the kind as an u32. This is used in the versioning arrays, and in
  /// plugin code, so that ints can be passed around instead of enums.
  pub fn id(self) -> u32 { ToPrimitive::to_u32(&self).unwrap() }
  /// Converts the given number to a block kind. If the number is invalid, this
  /// returns Kind::Air.
  pub fn from_u32(id: u32) -> Self { FromPrimitive::from_u32(id).unwrap_or(Type::Air) }
}

#[cfg(test)]
mod tests {

  #[test]
  fn test_blocks() {
    // TODO: Re-enable when items are re-added
    /*
    let data = generate_items();

    // Sanity check some random blocks
    assert_eq!(data[0].block_to_place, block::Kind::Air);
    assert_eq!(data[1].block_to_place, block::Kind::Stone);
    assert_eq!(data[2].block_to_place, block::Kind::Granite);
    assert_eq!(data[182].block_to_place, block::Kind::DiamondBlock);
    assert_eq!(data[430].block_to_place, block::Kind::Observer);
    // Used to show debug output.
    // assert!(false);
    */
  }
}

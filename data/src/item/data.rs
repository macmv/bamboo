use crate::block;

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

/// Generates a table from all items to any metadata that type has. This
/// includes things like the display name, stack size, etc.
pub fn generate_items() -> Vec<Data> {
  let mut items = vec![];
  include!(concat!(env!("OUT_DIR"), "/item/data.rs"));
  items
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

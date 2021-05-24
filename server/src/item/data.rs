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
  fn test_generate() {
    dbg!(generate_items());
    // Used to show debug output.
    // assert!(false);
  }
}

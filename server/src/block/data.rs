use super::{Kind, Type};
use std::collections::HashMap;

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  state:         u32,
  // A list of types in order. This will always be at least one element long.
  types:         Vec<Type>,
  // The default type. This is an index into types.
  default_index: u32,
}

/// Generates a table from all block kinds to any block data that kind has. This
/// does not include cross-versioning data. This includes information like the
/// block states, the properties it might have, and custom handlers for when the
/// block is place (things like making fences connect, or making stairs rotate
/// correctly).
///
/// This should only be called once, and will be done internally in the
/// [`WorldManager`](crate::world::WorldManager). This is left public as it may
/// be moved to a seperate crate in the future, as it takes a long time to
/// generate the source files for this.
///
/// Most of this function is generated at compile time. See
/// `gens/src/block/mod.rs` and `build.rs` for more.
pub fn generate_data() -> HashMap<Kind, Data> {
  let mut blocks = HashMap::new();
  include!(concat!(env!("OUT_DIR"), "/block/data.rs"));
  blocks
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate() {
    dbg!(generate_data());
    // Used to show debug output.
    // assert!(false);
  }
}

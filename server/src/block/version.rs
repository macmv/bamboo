use super::{Kind, Type};
use std::{collections::HashMap, io::BufReader};

use common::version::BlockVersion;

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Version {
  to_old: Vec<u32>,
  to_new: HashMap<u32, u32>,
  ver:    BlockVersion,
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
pub fn generate_versions() -> HashMap<BlockVersion, Version> {
  let mut versions = vec![];
  let csv = include_str!(concat!(env!("OUT_DIR"), "/block/versions.csv"));
  for (i, l) in csv.lines().enumerate() {
    let sections = l.split(',');
    if i == 0 {
      for _ in sections {
        versions.push(Version {
          to_old: vec![],
          to_new: HashMap::new(),
          ver:    BlockVersion::V1_8,
        });
      }
    } else {
      for (j, s) in sections.enumerate() {
        let v = s.parse().unwrap();
        versions[j].to_old.push(v);
        versions[j].to_new.insert(v, i as u32);
      }
    }
  }

  HashMap::new()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate() {
    dbg!(generate_versions());
    // Used to show debug output.
    // assert!(false);
  }
}

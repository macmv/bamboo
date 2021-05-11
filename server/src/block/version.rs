use std::collections::HashMap;

use common::version::BlockVersion;

pub struct Converter {
  versions: Vec<Version>,
}

impl Converter {
  pub fn new() -> Self {
    Self { versions: generate_versions() }
  }

  pub fn to_latest(&self, id: u32, ver: BlockVersion) -> u32 {
    match self.versions[ver.to_index() as usize].to_new.get(&id) {
      Some(v) => *v,
      None => 0,
    }
  }

  pub fn to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    match self.versions[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }
}

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
///
/// This Vec<Version> is in order of block versions. Use
/// BlockVersion::from_index() and BlockVersion::to_index() to convert between
/// indicies and block versions.
pub fn generate_versions() -> Vec<Version> {
  let mut versions = vec![];
  let csv = include_str!(concat!(env!("OUT_DIR"), "/block/versions.csv"));
  for (i, l) in csv.lines().enumerate() {
    let sections = l.split(',');
    if i == 0 {
      for (j, _) in sections.enumerate() {
        let ver = BlockVersion::from_index(j as u32);
        versions.push(Version { to_old: vec![], to_new: HashMap::new(), ver });
      }
    } else {
      for (j, s) in sections.enumerate() {
        let v = s.parse().unwrap();
        versions[j].to_old.push(v);
        versions[j].to_new.insert(v, i as u32);
      }
    }
  }

  versions
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

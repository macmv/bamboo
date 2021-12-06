use super::{ty, Data, Type};

use sc_common::version::BlockVersion;

/// This is a version converter. It is how all item ids are converter between
/// versions. This is much simpler than block conversion, as there are not
/// multiple states.
pub struct TypeConverter {
  types:    &'static [Data],
  versions: &'static [Version],
}

impl TypeConverter {
  /// Creates a new converter. This will parse the csv file, and allocate around
  /// 200K of memory. Do not call this unless you have a very good reason.
  /// Instead, use
  /// [`WorldManager::get_item_converter`](crate::world::WorldManager::
  /// get_item_converter).
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    // Self { types: ty::generate_items(), versions: generate_versions() }
    Self { types: &[], versions: &[] }
  }

  /// Takes the given old item id, which is part of `ver`, and returns the new
  /// id that it maps to. If the id is invalid, this will return 0 (empty).
  pub fn to_latest(&self, id: u32, ver: BlockVersion) -> u32 {
    // Air always maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.versions[self.versions.len() - ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Takes the new item id, and converts it to the old id, for the given
  /// version. If the id is invalid, this will return 0 (empty).
  pub fn to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.versions[self.versions.len() - ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Returns any data about this item. Includes things like max stack size,
  /// display name, etc.
  pub fn get_data(&self, item: Type) -> &Data { &self.types[item.to_u32() as usize] }
}

/// This is the conversion table for a single old version of the game and the
/// latest version. This includes a list of old ids, whose index is the new
/// block id. It also contains a HashMap, which is used to convert old ids into
/// new ones. This might not be fastest or most memory efficient way, but it is
/// certainly the easiest. Especially before 1.13, block ids are very sparse,
/// and a HashMap will be the best option.
#[derive(Debug)]
pub struct Version {
  // Index is the new id, value is the old id
  to_old: &'static [u32],
  // Index is the old id, value is the new id (0 for invalid)
  to_new: &'static [u32],
  ver:    BlockVersion,
}

include!(concat!(env!("OUT_DIR"), "/item/version.rs"));

/*
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
  let csv = include_str!(concat!(env!("OUT_DIR"), "/item/versions.csv"));
  for (i, l) in csv.lines().enumerate() {
    let mut sections = l.split(',').enumerate();
    // Remove the first element. This is the latest block id, which will always be
    // the same as i.
    sections.next().unwrap();
    if i == 0 {
      for (j, _) in sections {
        let ver = BlockVersion::from_index_rev(j as u32);
        versions.push(Version { to_old: vec![0], to_new: [(0, 0)].iter().cloned().collect(), ver });
      }
    } else {
      for (j, s) in sections {
        let v = s.parse().unwrap();
        // This versions list doesn't contain latest, so we have to subtract one.
        versions[j - 1].to_old.push(v);
        versions[j - 1].to_new.insert(v, i as u32);
      }
    }
  }

  // In this list, versions[0] is the latest version. The way the BlockVersion
  // enum is built, this is backwards. So we reverse it here, so that the types
  // using this list can just call BlockVersion::to_index().
  versions.into_iter().rev().collect()
}
*/

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_versioning() {
    let conv = TypeConverter::new();

    // Diamond block
    assert_eq!(conv.to_old(182, BlockVersion::V1_8), 57);
  }
}

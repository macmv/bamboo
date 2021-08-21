use std::collections::HashMap;

use super::{ty, ty::Version, Data, Kind};

use common::version::BlockVersion;

/// This is a version converter. It is how all block ids are converter between
/// versions. At compile time, all of the vanilla Minecraft versions that are
/// supported are processed and converted into various files. One of these files
/// is a csv file, which is embedded into this binary. This csv contains a list
/// of all modern blocks within the game, and the ids that those ids map to in
/// all older versions of the game. This file is quite large, and is parsed any
/// time one of these is created. This type ends up using about 600K of memory,
/// so you should only ever create one. The
/// [`WorldManager`](crate::world::WorldManager) has one of these which you can
/// use.
///
/// This type does not implement [`Default`], because it is very expensive to
/// create one of these. A new one should only be constructed explicitly, and
/// should never be neccessary.
pub struct TypeConverter {
  // Each index into the outer vec is a kind id. Indexing into the inner vec is each variant of the
  // given kind. These are in such an order that iterating through both of them will get all block
  // types in the same order as the global palette of the latest version.
  kinds:    Vec<Data>,
  versions: &'static [Version],
}

impl TypeConverter {
  /// Creates a new converter. This will parse the csv file, and allocate around
  /// 600K of memory. Do not call this unless you have a very good reason.
  /// Instead, use
  /// [`WorldManager::get_converter`](crate::world::WorldManager::
  /// get_converter).
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self { kinds: ty::generate_kinds(), versions: ty::generate_versions() }
  }

  /// Takes the given old block id, which is part of `ver`, and returns the new
  /// id that it maps to. If the id is invalid, this will return 0 (air).
  pub fn to_latest(&self, id: u32, ver: BlockVersion) -> u32 {
    match self.versions[ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Takes the new block id, and converts it to the old id, for the given
  /// version. If the id is invalid, this will return 0 (air).
  pub fn to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.versions[ver.to_index() as usize - 1].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Gets all the data for a given block kind. This includes all the types, the
  /// default type, and state ids.
  pub fn get(&self, k: Kind) -> &Data {
    &self.kinds[k.id() as usize]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_convert() {
    let conv = TypeConverter::new();

    // This line makes it easy to test each version
    // 15743,11268,11252,8595,4080,4080,4080,4080,0

    assert_eq!(conv.to_old(15743, BlockVersion::V1_16), 15743);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_15), 11268);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_14), 11252);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_13), 8595);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_12), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_11), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_10), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_9), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_8), 0);

    // Used to show debug output.
    // assert!(false);
  }
}

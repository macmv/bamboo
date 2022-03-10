use super::{ty, Data, Kind, Type};
use sc_common::version::BlockVersion;

/// This is the conversion table for a single old version of the game and the
/// latest version. This includes a list of old ids, whose index is the new
/// block id. It also contains a HashMap, which is used to convert old ids into
/// new ones. This might not be fastest or most memory efficient way, but it is
/// certainly the easiest. Especially before 1.13, block ids are very sparse,
/// and a HashMap will be the best option.
#[derive(Debug)]
pub struct Version {
  to_old: &'static [u32],
  to_new: &'static [u32],
  #[allow(unused)]
  ver:    BlockVersion,
}

include!(concat!(env!("OUT_DIR"), "/block/version.rs"));

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
  kinds:        &'static [Data],
  versions:     &'static [Version],
  // A list of all latest block states, and which kind they map to.
  block_states: &'static [Kind],
}

impl TypeConverter {
  /// Creates a new converter. This will parse the csv file, and allocate around
  /// 600K of memory. Do not call this unless you have a very good reason.
  /// Instead, use
  /// [`WorldManager::get_converter`](crate::world::WorldManager::
  /// get_converter).
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    let (kinds, block_states) = ty::generate_kinds();
    Self { kinds, versions: generate_versions(), block_states }
  }

  /// Takes the given old block id, which is part of `ver`, and returns the new
  /// id that it maps to. If the id is invalid, this will return 0 (air).
  pub fn to_latest(&self, id: u32, ver: BlockVersion) -> u32 {
    // Air always maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
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
    match self.versions[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Gets all the data for a given block kind. This includes all the types, the
  /// default type, and state ids.
  pub fn get(&self, k: Kind) -> &Data { &self.kinds[k.id() as usize] }

  /// Gets a block type from the given id.
  pub fn type_from_id(&self, mut id: u32, ver: BlockVersion) -> Type {
    if ver != BlockVersion::latest() {
      id = self.to_latest(id, ver);
    }

    let kind = self.kind_from_id(id, BlockVersion::latest());
    let data = self.get(kind);
    data.type_from_id(id - data.state)
  }

  /// Gets a block kind from the given id.
  pub fn kind_from_id(&self, mut id: u32, ver: BlockVersion) -> Kind {
    if ver != BlockVersion::latest() {
      id = self.to_latest(id, ver);
    }

    self.block_states.get(id as usize).copied().unwrap_or(Kind::Air)
  }

  pub fn max_bpe(&self) -> u8 { (self.block_states.len() as f32).log2().ceil() as u8 }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_convert() {
    let conv = TypeConverter::new();

    dbg!(BlockVersion::V1_15.to_index());
    for (i, v) in conv.versions.iter().enumerate() {
      dbg!(i, v.ver);
    }

    // Some easy conversions
    assert_eq!(conv.to_old(1, BlockVersion::V1_8), 1 << 4);

    // for i in 0..16000 {
    //   if conv.to_old(i, BlockVersion::V1_15) != 0 {
    //     dbg!(i);
    //   }
    // }

    // This line makes it easy to test each version
    // 15743,11268,11252,8595,4080,4080,4080,4080,0

    /*
    assert_eq!(conv.to_old(15743, BlockVersion::V1_16), 15743);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_15), 11268);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_14), 11252);
    // assert_eq!(conv.to_old(15743, BlockVersion::V1_13), 8595);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_12), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_11), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_10), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_9), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_8), 0);

    assert_eq!(conv.to_latest(15743, BlockVersion::V1_16), 15743);
    assert_eq!(conv.to_latest(11268, BlockVersion::V1_15), 15743);
    assert_eq!(conv.to_latest(11252, BlockVersion::V1_14), 15743);
    // assert_eq!(conv.to_latest(8595, BlockVersion::V1_13), 15743);
    assert_eq!(conv.to_latest(4080, BlockVersion::V1_12), 15743);
    assert_eq!(conv.to_latest(4080, BlockVersion::V1_11), 15743);
    assert_eq!(conv.to_latest(4080, BlockVersion::V1_10), 15743);
    assert_eq!(conv.to_latest(4080, BlockVersion::V1_9), 15743);
    assert_eq!(conv.to_latest(0, BlockVersion::V1_8), 0);
    */

    assert_eq!(conv.kind_from_id(15743, BlockVersion::V1_16), Kind::StructureBlock);
    assert_eq!(conv.kind_from_id(11268, BlockVersion::V1_15), Kind::StructureBlock);
    assert_eq!(conv.kind_from_id(11252, BlockVersion::V1_14), Kind::StructureBlock);
    /*
    assert_eq!(conv.kind_from_id(8595, BlockVersion::V1_13), Kind::StructureBlock);
    */
    assert_eq!(conv.kind_from_id(4080, BlockVersion::V1_12), Kind::StructureBlock);
    assert_eq!(conv.kind_from_id(4080, BlockVersion::V1_11), Kind::StructureBlock);
    assert_eq!(conv.kind_from_id(4080, BlockVersion::V1_10), Kind::StructureBlock);
    assert_eq!(conv.kind_from_id(4080, BlockVersion::V1_9), Kind::StructureBlock);
    assert_eq!(conv.kind_from_id(0, BlockVersion::V1_8), Kind::Air);

    // Used to show debug output.
    // assert!(false);
  }
}

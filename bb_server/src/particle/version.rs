use super::{ty, Data, Type};

use bb_common::version::BlockVersion;

/// This is a version converter. It is how all item ids are converter between
/// versions. This is much simpler than block conversion, as there are not
/// multiple states.
pub struct TypeConverter {
  types:    &'static [Data],
  versions: &'static [Version],
}

impl TypeConverter {
  /// Creates a new converter. This will allocate a bunch. Do not call this
  /// unless you have a very good reason. Instead, use
  /// [`WorldManager::particle_converter`].
  ///
  /// [`WorldManager::particle_converter`]: crate::world::WorldManager::particle_converter
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { Self { types: ty::generate_kinds(), versions: generate_versions() } }

  /// Takes the given old particle id, which is part of `ver`, and returns the
  /// new id that it maps to. If the id is invalid, this will return `None`.
  pub fn to_latest(&self, id: u32, ver: BlockVersion) -> Option<u32> {
    if ver == BlockVersion::latest() {
      return Some(id);
    }
    self.versions[self.versions.len() - ver.to_index() as usize]
      .to_new
      .get(id as usize)
      .copied()
      .unwrap_or(None)
  }

  /// Takes the new particle id, and converts it to the old id, for the given
  /// version. If the id is invalid, this will return `None`.
  pub fn to_old(&self, id: u32, ver: BlockVersion) -> Option<u32> {
    if ver == BlockVersion::latest() {
      return Some(id);
    }
    self.versions[self.versions.len() - ver.to_index() as usize]
      .to_old
      .get(id as usize)
      .copied()
      .unwrap_or(None)
  }

  /// Returns any data about this item. Includes things like max stack size,
  /// display name, etc.
  pub fn get_data(&self, item: Type) -> &Data { &self.types[item.id() as usize] }
}

/// This is the conversion table for a single old version of the game and the
/// latest version. This includes a list of old ids, whose index is the new
/// block id. It also contains a HashMap, which is used to convert old ids into
/// new ones. This might not be fastest or most memory efficient way, but it is
/// certainly the easiest. Especially before 1.13, block ids are very sparse,
/// and a HashMap will be the best option.
#[derive(Debug)]
pub struct Version {
  // Index is the new id, value is the (old id, old damage)
  to_old: &'static [Option<u32>],
  // Index is the old id, value is a list of old damage to new id (0 for invalid)
  to_new: &'static [Option<u32>],
  #[allow(unused)]
  ver:    BlockVersion,
}

include!(concat!(env!("OUT_DIR"), "/particle/version.rs"));

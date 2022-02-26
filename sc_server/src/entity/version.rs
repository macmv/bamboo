use super::{ty, Data, Type};
use sc_common::version::BlockVersion;

/// This is a version converter. It is how all entity ids are converted between
/// versions. This is much simpler than block conversion, as there are not
/// multiple states for each entity.
pub struct TypeConverter {
  types:    &'static [Data],
  versions: &'static [Version],
}

impl TypeConverter {
  /// Creates a new converter. This will reload all the versioning data. This is
  /// mostly built into the binary, but it is still a waste to call this
  /// function. Do not call this unless you have a very good reason. Instead,
  /// use [`WorldManager::get_entity_converter`](crate::world::WorldManager::
  /// get_entity_converter).
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { Self { types: ty::generate_kinds(), versions: generate_versions() } }

  /// Takes the given old entity id, which is part of `ver`, and returns the new
  /// id that it maps to. If the id is invalid, this will make a guess at what
  /// entity is most similar. If it fails, it will return 0.
  pub fn to_latest(&self, id: u32, ver: BlockVersion) -> u32 {
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

  /// Takes the new entity id, and converts it to the old id, for the given
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
  pub fn get_data(&self, entity: Type) -> &Data { &self.types[entity.id() as usize] }
}

#[derive(Debug)]
pub struct Version {
  to_old: &'static [u32],
  to_new: &'static [u32],
  #[allow(unused)]
  ver:    BlockVersion,
}

include!(concat!(env!("OUT_DIR"), "/entity/version.rs"));

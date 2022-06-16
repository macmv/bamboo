use super::{ty, Data, Type};
use bb_common::version::BlockVersion;

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
  /// use [`WorldManager::entity_converter`].
  ///
  /// [`WorldManager::entity_converter`]: crate::world::WorldManager::entity_converter
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

// TODO: Don't include versioning data, as we don't really need it.

#[derive(Debug)]
pub struct Version {
  to_old:   &'static [u32],
  to_new:   &'static [u32],
  // These are required for the data generator.
  #[allow(unused)]
  metadata: &'static [Metadata],
  #[allow(unused)]
  ver:      BlockVersion,
}

include!(concat!(env!("OUT_DIR"), "/entity/version.rs"));

#[derive(Debug, Clone)]
pub struct Metadata {
  #[allow(unused)]
  to_old:    &'static [u8],
  #[allow(unused)]
  to_new:    &'static [u8],
  #[allow(unused)]
  old_types: &'static [Option<MetadataType>],
  #[allow(unused)]
  new_types: &'static [MetadataType],
}

/// An entity metadata type. Note that the documentation for this type is for
/// 1.18.2. Older versions will have different serializing/deserializing rules.
#[derive(Debug, Clone)]
pub enum MetadataType {
  /// A single byte.
  Byte,
  /// A varint (same as protocol).
  VarInt,
  /// A short. Only present on 1.8-1.12.
  Short,
  /// A 4 byte floating point number
  Float,
  /// A varint prefixed string
  String,
  /// A string, which is JSON encoded chat data.
  Chat,
  /// A boolean. If true, this is followed by a Chat field.
  OptChat,
  /// An item stack. Same as protocol.
  Item,
  /// A single byte.
  Bool,
  /// 3 floats for X, Y, then Z.
  Rotation,
  /// A position encoded as a long.
  Position,
  /// A boolean. If true, this is followed by a Position.
  OptPosition,
  /// A VarInt. This will be from 0-5 (inclusive), which maps to a direction
  /// like so:
  /// - 0: Down
  /// - 1: Up
  /// - 2: North
  /// - 3: South
  /// - 4: West
  /// - 5: East
  Direction,
  /// A boolean. If true, then a 16 byte UUID follows.
  OptUUID,
  /// A varint, which should be parsed as a block ID.
  BlockID,
  /// An NBT tag. This is not length prefixed. The entire tag must be parsed to
  /// find the end of this field.
  NBT,
  /// A VarInt for the particle ID, followed by some data. The data following
  /// must be infered from the particle ID.
  Particle,
  /// 3 VarInts: villager type, villager profession, and villager level.
  VillagerData,
  /// A boolean. If true, a VarInt follows.
  OptVarInt,
  /// A VarInt, from 0-7 (inclusive). The numbers map to these poses:
  /// - 0: Standing
  /// - 1: Fall flying
  /// - 2: Pdeeping
  /// - 3: Swiming
  /// - 4: Spin attack
  /// - 5: Sneaking
  /// - 6: Long jumping
  /// - 7: Dying
  Pose,

  /// TODO: Figure out what this is!
  FireworkData,
  /// A varint
  CatVariant,
  /// A varint
  FrogVariant,
  /// A varint
  PaintingVariant,
}

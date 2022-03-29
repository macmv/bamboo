use sc_common::version::BlockVersion;

pub struct TypeConverter {
  blocks:   &'static [block::Version],
  items:    &'static [item::Version],
  entities: &'static [entity::Version],
}

mod block {
  use sc_common::version::BlockVersion;

  #[derive(Debug)]
  pub struct Version {
    pub to_old: &'static [u32],
    pub to_new: &'static [u32],
    pub ver:    BlockVersion,
  }

  include!(concat!(env!("OUT_DIR"), "/block/version.rs"));
}

mod item {
  use sc_common::version::BlockVersion;

  #[derive(Debug)]
  pub struct Version {
    pub to_old: &'static [(u32, u32)],
    pub to_new: &'static [&'static [u32]],
    pub ver:    BlockVersion,
  }

  include!(concat!(env!("OUT_DIR"), "/item/version.rs"));
}

pub(super) mod entity {
  use sc_common::version::BlockVersion;

  #[derive(Debug)]
  pub struct Version {
    pub to_old:   &'static [u32],
    pub to_new:   &'static [u32],
    /// List of new entities, indexed by modern ids (no matter what `ver` is).
    pub metadata: &'static [Metadata],
    pub ver:      BlockVersion,
  }

  include!(concat!(env!("OUT_DIR"), "/entity/version.rs"));

  pub use super::entity_types::*;
}

impl TypeConverter {
  pub fn new() -> Self {
    TypeConverter {
      blocks:   block::generate_versions(),
      items:    item::generate_versions(),
      entities: entity::generate_versions(),
    }
  }
}

impl TypeConverter {
  /// The `id` argument is a block id in the given version. The returned block
  /// id should be the equivalent id in the latest version this server supports.
  /// This should also support passing in the latest version (it should return
  /// the same id).
  pub fn block_to_new(&self, id: u32, ver: BlockVersion) -> u32 {
    // Air always maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.blocks[ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }
  /// The `id` argument is a block id in the latest version. This function
  /// should return the equivalent block id for the given version. It should
  /// also work when passed the latest version (it should return the same id).
  pub fn block_to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.blocks[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Converts an item id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  pub fn item_to_new(&self, id: u32, damage: u32, ver: BlockVersion) -> u32 {
    // Air always maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.items[ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => v.get(damage as usize).copied().unwrap_or(0),
      None => 0,
    }
  }
  /// Converts an item id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  pub fn item_to_old(&self, id: u32, ver: BlockVersion) -> (u32, u32) {
    if ver == BlockVersion::latest() {
      return (id, 0);
    }
    match self.items[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => (0, 0),
    }
  }

  /// Converts an entity id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  pub fn entity_to_new(&self, id: u32, ver: BlockVersion) -> u32 {
    // Air alwas maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.entities[ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }
  /// Converts an entity id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  pub fn entity_to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.entities[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Converts an entity metadata field id into an id for the given version.
  ///
  /// The argument `ty` is the *modern* (not old) entity type. The argument `id`
  /// is the modern metadata field index. The argument `ver` is the version
  /// that `id` is being converted to.
  pub fn entity_metadata_to_old(&self, ty: u32, id: u8, ver: BlockVersion) -> u8 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.entities[ver.to_index() as usize].metadata[ty as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Returns the (new, old) entity metadata type for the given field. This is
  /// used to convert metadata fields into older versions.
  ///
  /// The argument `ty` is the *modern* (not old) entity type. The argument `id`
  /// is the modern metadata field index. The argument `ver` is the version
  /// that `id` is being converted to.
  pub fn entity_metadata_types(
    &self,
    ty: u32,
    id: u8,
    ver: BlockVersion,
  ) -> (u8, entity::MetadataType, entity::MetadataType) {
    let meta = &self.entities[ver.to_index() as usize].metadata[ty as usize];
    let old_id = meta.to_old.get(id as usize).copied().unwrap_or(0);
    (old_id, meta.new_types[id as usize], meta.old_types[old_id as usize].unwrap())
  }
}

mod entity_types {
  #[derive(Debug, Clone)]
  pub struct Metadata {
    pub to_old:    &'static [u8],
    pub to_new:    &'static [u8],
    pub old_types: &'static [Option<MetadataType>],
    pub new_types: &'static [MetadataType],
  }

  /// An entity metadata type. Note that the documentation for this type is for
  /// 1.18.2. Older versions will have different serializing/deserializing
  /// rules.
  #[derive(Debug, Clone, Copy)]
  pub enum MetadataType {
    /// A single byte.
    Byte,
    /// A varint (same as protocol).
    VarInt,
    /// A short. Only present on 1.8-1.12.
    Short,
    /// An int. Only present on 1.8-1.12.
    Int,
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
    /// An NBT tag. This is not length prefixed. The entire tag must be parsed
    /// to find the end of this field.
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
    /// - 2: Sleeping
    /// - 3: Swiming
    /// - 4: Spin attack
    /// - 5: Sneaking
    /// - 6: Long jumping
    /// - 7: Dying
    Pose,

    /// TODO: Figure out what this is!
    FireworkData,
  }
}

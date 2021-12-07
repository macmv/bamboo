use sc_common::version::BlockVersion;

pub struct TypeConverter {
  blocks: &'static [block::Version],
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

impl TypeConverter {
  pub fn new() -> Self { TypeConverter { blocks: block::generate_versions() } }
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

  /// Converts an item id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  pub fn item_to_old(&self, id: u32, ver: BlockVersion) -> u32 { 0 }
  /// Converts an item id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  pub fn item_to_new(&self, id: u32, ver: BlockVersion) -> u32 { 0 }

  /// Converts an entity id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  pub fn entity_to_old(&self, id: u32, ver: BlockVersion) -> u32 { 0 }
  /// Converts an entity id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  pub fn entity_to_new(&self, id: u32, ver: BlockVersion) -> u32 { 0 }
}

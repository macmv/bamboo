use sc_common::{net::VersionConverter, version::BlockVersion};

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

impl VersionConverter for TypeConverter {
  fn block_to_new(&self, id: u32, ver: BlockVersion) -> u32 {
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

  /// Takes the new block id, and converts it to the old id, for the given
  /// version. If the id is invalid, this will return 0 (air).
  fn block_to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.blocks[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }
}

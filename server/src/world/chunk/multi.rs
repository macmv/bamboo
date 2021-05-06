use std::collections::HashMap;

use common::{math::Pos, version::BlockVersion};

use super::Chunk;
use crate::block;

pub struct MultiChunk {
  primary:  BlockVersion,
  versions: HashMap<BlockVersion, Chunk>,
}

impl MultiChunk {
  pub fn new() -> MultiChunk {
    let mut versions = HashMap::new();
    versions.insert(BlockVersion::V1_8, Chunk::new(BlockVersion::V1_8));

    MultiChunk { primary: BlockVersion::V1_8, versions }
  }

  pub fn set_block(&mut self, p: Pos, ty: &block::Type) {
    for (v, c) in self.versions.iter_mut() {
      c.set_block(p, ty);
    }
  }
}

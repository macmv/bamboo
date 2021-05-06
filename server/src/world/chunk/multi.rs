use std::collections::HashMap;

use common::{
  math::{Pos, PosError},
  proto,
  version::BlockVersion,
};

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

  /// Sets a block within this chunk. p.x and p.z must be within 0..16. If the
  /// server is only running on 1.17, then p.y needs to be within the world
  /// height (whatever that may be). Otherwise, p.y must be within 0..256.
  pub fn set_block(&mut self, p: Pos, ty: &block::Type) -> Result<(), PosError> {
    for (_, c) in self.versions.iter_mut() {
      c.set_block(p, ty)?;
    }
    Ok(())
  }

  /// Gets the type of a block within this chunk. Pos must be within the chunk.
  /// See [`set_block`](Self::set_block) for more.
  pub fn get_block(&self, p: Pos) -> Result<block::Type, PosError> {
    self.versions[&self.primary].get_block(p)
  }

  /// Generates a protobuf for the given version. The proto's X and Z
  /// coordinates must be sent before it can be sent over a grpc connection.
  pub fn to_proto(&self, v: BlockVersion) -> proto::Chunk {
    self.versions[&v].to_proto()
  }
}

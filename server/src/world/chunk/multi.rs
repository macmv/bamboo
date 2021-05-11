use std::{collections::HashMap, sync::Arc};

use common::{
  math::{Pos, PosError},
  proto,
  version::BlockVersion,
};

use super::Chunk;
use crate::block;

pub struct MultiChunk {
  inner:     Chunk,
  converter: Arc<block::Converter>,
}

impl MultiChunk {
  /// Creates an empty chunk. Currently, it just creates a seperate chunk for
  /// every supported version. In the future, it will take a list of versions as
  /// parameters. If it is fast enough, I might generate a mapping of all new
  /// block ids and how they can be transformed into old block ids. Then, this
  /// would only store one chunk, and would perform all conversions when you
  /// actually tried to get an old id.
  pub fn new(converter: Arc<block::Converter>) -> MultiChunk {
    let mut versions = HashMap::new();
    versions.insert(BlockVersion::V1_8, Chunk::new(BlockVersion::V1_8));

    MultiChunk { inner: Chunk::new(BlockVersion::latest()), converter }
  }

  /// Sets a block within this chunk. p.x and p.z must be within 0..16. If the
  /// server is only running on 1.17, then p.y needs to be within the world
  /// height (whatever that may be). Otherwise, p.y must be within 0..256.
  pub fn set_block(&mut self, p: Pos, ty: &block::Type) -> Result<(), PosError> {
    self.inner.set_block(p, ty.id())
  }

  /// Gets the type of a block within this chunk. Pos must be within the chunk.
  /// See [`set_block`](Self::set_block) for more.
  ///
  /// This will return a blockid. This block id is from the primary version of
  /// this chunk. That can be known by calling [`primary`](Self::primary). It
  /// will usually be the latest version that this server supports. Regardless
  /// of what it is, this should be handled within the World.
  pub fn get_block(&self, p: Pos) -> Result<u32, PosError> {
    self.inner.get_block(p)
  }

  /// Generates a protobuf for the given version. The proto's X and Z
  /// coordinates are 0.
  pub fn to_proto(&self, v: BlockVersion) -> proto::Chunk {
    if v == BlockVersion::latest() {
      self.inner.to_latest_proto()
    } else {
      self.inner.to_old_proto(|id| self.converter.to_old(id, v))
    }
  }
}

use std::{collections::HashMap, sync::Arc};

use common::{
  math::{Pos, PosError},
  proto,
  version::BlockVersion,
};

use super::Chunk;
use crate::block;

pub struct MultiChunk {
  inner: Chunk,
  types: Arc<block::TypeConverter>,
}

impl MultiChunk {
  /// Creates an empty chunk. Currently, it just creates a seperate chunk for
  /// every supported version. In the future, it will take a list of versions as
  /// parameters. If it is fast enough, I might generate a mapping of all new
  /// block ids and how they can be transformed into old block ids. Then, this
  /// would only store one chunk, and would perform all conversions when you
  /// actually tried to get an old id.
  pub fn new(types: Arc<block::TypeConverter>) -> MultiChunk {
    let mut versions = HashMap::new();
    versions.insert(BlockVersion::V1_8, Chunk::new(BlockVersion::V1_8));

    MultiChunk { inner: Chunk::new(BlockVersion::latest()), types }
  }

  /// Sets a block within this chunk. p.x and p.z must be within 0..16. If the
  /// server is only running on 1.17, then p.y needs to be within the world
  /// height (whatever that may be). Otherwise, p.y must be within 0..256.
  pub fn set_type(&mut self, p: Pos, ty: &block::Type) -> Result<(), PosError> {
    self.inner.set_block(p, ty.id())
  }

  /// Sets a block within this chunk. This is the same as
  /// [`set_type`](Self::set_type), but it uses a kind instead of a type. This
  /// will use the default type of the given kind.
  pub fn set_kind(&mut self, p: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.inner.set_block(p, self.types.get(kind).default_type().id())
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
      self.inner.to_old_proto(|id| self.types.to_old(id, v))
    }
  }
}

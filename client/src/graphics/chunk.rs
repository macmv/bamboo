use common::{
  chunk::{Chunk, ChunkKind},
  proto,
};
use std::ops::Deref;

/// A chunk with a mesh. This acts the same as a normal mesh, but will lazily
/// update a mesh any time it needs to be rendered.
pub struct MeshChunk {
  chunk: Chunk,
}

impl MeshChunk {
  /// Creates a new mesh chunk from the given chunk. This will generate all of
  /// the initial geometry for this chunk. Any time this chunk is rendered, the
  /// geometry will be updated (not when the chunk itself is updated).
  pub fn new(chunk: Chunk) -> Self {
    MeshChunk { chunk }
  }

  /// Creates a new mesh chunk from the given protobuf. This will call
  /// [`new`](Self::new) after parsing the protobuf.
  pub fn from_proto(p: proto::Chunk) -> Self {
    MeshChunk::new(Chunk::from_latest_proto(p, ChunkKind::Fixed))
  }
}

impl Deref for MeshChunk {
  type Target = Chunk;

  fn deref(&self) -> &Self::Target {
    &self.chunk
  }
}

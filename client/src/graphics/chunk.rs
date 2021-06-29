use common::{
  chunk::{Chunk, ChunkKind},
  proto,
};

/// A chunk with a mesh. This acts the same as a normal mesh, but will lazily
/// update a mesh any time it needs to be rendered.
pub struct MeshChunk {
  chunk: Chunk,
}

impl MeshChunk {
  /// Creates a new mesh chunk from the given chunk. This will generate all of
  /// the initial geometry for this chunk. Any time this chunk is updated, the
  /// geometry will be updated at the same time (not on another thread).
  pub fn new(chunk: Chunk) -> Self {
    MeshChunk { chunk }
  }

  /// Creates a new mesh chunk from the given protobuf. This will call
  /// [`new`](Self::new) after parsing the protobuf.
  pub fn from_proto(p: proto::Chunk) -> Self {
    MeshChunk::new(Chunk::from_latest_proto(p, ChunkKind::Fixed))
  }
}

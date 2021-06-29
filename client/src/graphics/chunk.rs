use crate::graphics::{Vert3, WindowData};
use common::{
  chunk::{Chunk, ChunkKind},
  proto,
};
use std::{ops::Deref, sync::Arc};

use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  device::Device,
};

/// A chunk with a mesh. This acts the same as a normal mesh, but will lazily
/// update a mesh any time it needs to be rendered.
pub struct MeshChunk {
  chunk: Chunk,
  vbuf:  Arc<CpuAccessibleBuffer<[Vert3]>>,
}

impl MeshChunk {
  /// Creates a new mesh chunk from the given chunk. This will generate all of
  /// the initial geometry for this chunk. Any time this chunk is rendered, the
  /// geometry will be updated (not when the chunk itself is updated).
  pub fn new(chunk: Chunk, device: Arc<Device>) -> Self {
    let vbuf =
      CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), false, [].iter().cloned())
        .unwrap();
    MeshChunk { chunk, vbuf }
  }

  /// Returns the buffer used to render this chunk. This will also update this
  /// buffer if the geometry is out of date.
  pub fn get_vbuf(&self) -> &Arc<CpuAccessibleBuffer<[Vert3]>> {
    // TODO: Generate mesh
    &self.vbuf
  }

  /// Creates a new mesh chunk from the given protobuf. This will call
  /// [`new`](Self::new) after parsing the protobuf.
  pub fn from_proto(p: proto::Chunk, device: Arc<Device>) -> Self {
    MeshChunk::new(Chunk::from_latest_proto(p, ChunkKind::Fixed), device)
  }
}

impl Deref for MeshChunk {
  type Target = Chunk;

  fn deref(&self) -> &Self::Target {
    &self.chunk
  }
}

use crate::graphics::{Vert3, WindowData};
use common::{
  chunk::{Chunk, ChunkKind},
  math::{Pos, PosError},
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
  chunk:    Chunk,
  outdated: bool,
  vbuf:     Arc<CpuAccessibleBuffer<[Vert3]>>,
}

impl MeshChunk {
  /// Creates a new mesh chunk from the given chunk. This will generate all of
  /// the initial geometry for this chunk. Any time this chunk is rendered, the
  /// geometry will be updated (not when the chunk itself is updated).
  pub fn new(chunk: Chunk, device: Arc<Device>) -> Self {
    let vbuf =
      CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), false, [].iter().cloned())
        .unwrap();
    let mut c = MeshChunk { chunk, vbuf, outdated: true };
    c.update_mesh();
    c
  }

  /// Returns the buffer used to render this chunk. This will also update this
  /// buffer if the geometry is out of date.
  pub fn get_vbuf(&mut self) -> &Arc<CpuAccessibleBuffer<[Vert3]>> {
    if self.outdated {
      self.update_mesh();
    }
    // TODO: Generate mesh
    &self.vbuf
  }

  /// Updates the mesh. This should only be called internally, but if called
  /// externally, the new mesh will be correctly used on the next frame.
  pub fn update_mesh(&mut self) {
    for y in 0..256 {
      for z in 0..16 {
        for x in 0..16 {
          if self.chunk.get_block(Pos::new(x, y, z)) != Ok(0) {
            // We only want to check for solids next to air, not the other way around.
            continue;
          }
          let up = self.chunk.get_block(Pos::new(x, y, z)) != Ok(0);
        }
      }
    }
    self.outdated = false;
  }

  /// Creates a new mesh chunk from the given protobuf. This will call
  /// [`new`](Self::new) after parsing the protobuf.
  pub fn from_proto(p: proto::Chunk, device: Arc<Device>) -> Self {
    MeshChunk::new(Chunk::from_latest_proto(p, ChunkKind::Fixed), device)
  }

  // Overrides the [`set_block`](Chunk::set_block) function on [`Chunk`]. This is
  // done so that the mesh will be correctly updated on the next frame.
  pub fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    self.outdated = true;
    self.chunk.set_block(pos, ty)
  }

  // Overrides the [`fill`](Chunk::fill) function on [`Chunk`]. This is done so
  // that the mesh will be correctly updated on the next frame.
  pub fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError> {
    self.outdated = false;
    self.chunk.fill(min, max, ty)
  }
}

impl Deref for MeshChunk {
  type Target = Chunk;

  fn deref(&self) -> &Self::Target {
    &self.chunk
  }
}

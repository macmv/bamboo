use super::WorldManager;
use crate::{
  block,
  block::{
    light::{BlockLight, SkyLight},
    TileEntity,
  },
};
use bb_common::{
  chunk::{paletted::Section as PalettedSection, Chunk, LightChunk},
  math::{PosError, RelPos},
  version::BlockVersion,
};
use parking_lot::{Mutex, MutexGuard};
use std::{
  collections::HashMap,
  fmt,
  sync::{atomic::AtomicU32, Arc},
};

/// A chunk in the world with a number of people viewing it. If the count is at
/// 0, then this chunk is essentially flagged for unloading. Chunks are unloaded
/// lazily, so this chunk will just end up being cleaned up in the future.
pub struct CountedChunk {
  pub(super) count: AtomicU32,
  pub chunk:        Mutex<MultiChunk>,
}

impl fmt::Debug for CountedChunk {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("CountedChunk").field("count", &self.count).finish()
  }
}

/// This stores the block information for the latest version, block lighting
/// information, and optionally sky light information.
///
/// In the past, this used to store a copy of the chunk data for each version.
/// However, converting the palette on the proxy with a lookup table ended up
/// using far less memory, and was nearly as fast. The name starts with Multi
/// because it used to store all of the other versioning data. It would be a
/// pain to change it, and I don't really want to bother.
pub struct MultiChunk {
  inner: ChunkData,
  wm:    Arc<WorldManager>,
}

struct ChunkData {
  inner:        Chunk<PalettedSection>,
  tes:          HashMap<RelPos, Arc<dyn TileEntity>>,
  sky:          Option<LightChunk<SkyLight>>,
  block:        LightChunk<BlockLight>,
  /// Set to false when the world is generating, which makes things much faster.
  update_light: bool,

  height: u32,
  min_y:  i32,
}

impl fmt::Debug for MultiChunk {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.debug_struct("MultiChunk").finish() }
}

impl CountedChunk {
  /// Creates a new counted chunk with the counter at 0.
  pub fn new(c: MultiChunk) -> CountedChunk {
    CountedChunk { count: 0.into(), chunk: Mutex::new(c) }
  }
  pub fn lock(&self) -> MutexGuard<'_, MultiChunk> { self.chunk.lock() }
}

impl ChunkData {
  fn new(sky: bool, height: u32, min_y: i32) -> Self {
    ChunkData {
      inner: Chunk::new(15),
      tes: HashMap::new(),
      sky: if sky { Some(LightChunk::new()) } else { None },
      block: LightChunk::new(),
      update_light: true,
      height,
      min_y,
    }
  }

  /// A `Type<'a>` borrows `self`, so we can't pass that into `set_type`.
  /// Therefore, we use this inner function to avoid allocating a `TypeStore` in
  /// `set_kind`.
  fn set_type_id(
    &mut self,
    p: RelPos,
    ty: u32,
    kind: block::Kind,
    behaviors: &block::BehaviorStore,
  ) -> Result<(), PosError> {
    self.inner.set_block(p, ty)?;
    self.update_light(p);
    if let Some(te) = behaviors.call(kind, |b| b.create_te()) {
      self.tes.insert(p, te);
    }
    Ok(())
  }

  fn enable_lighting(&mut self, enabled: bool) {
    if !self.update_light && enabled {
      self.update_all_light();
    }
    self.update_light = enabled;
  }

  fn update_all_light(&mut self) {
    if let Some(sky) = &mut self.sky {
      sky.update_all(&self.inner);
    }
    self.block.update_all(&self.inner);
  }
  fn update_light(&mut self, pos: RelPos) {
    if self.update_light {
      if let Some(sky) = &mut self.sky {
        sky.update(&self.inner, pos);
      }
      self.block.update(&self.inner, pos);
    }
  }
}

impl MultiChunk {
  /// Creates an empty chunk.
  ///
  /// The second argument is for sky light data. Places like the nether do not
  /// contain sky light information, so the sky light data is not present.
  pub fn new(wm: Arc<WorldManager>, sky: bool, height: u32, min_y: i32) -> MultiChunk {
    MultiChunk { inner: ChunkData::new(sky, height, min_y), wm }
  }

  /// Returns a reference to the global world manager.
  pub fn wm(&self) -> &Arc<WorldManager> { &self.wm }

  /// Sets a block within this chunk. `p.x` and `p.z` must be within 0..16. If
  /// the server supports multi-height worlds (not implemented yet), then p.y
  /// needs to be within the world height (whatever that may be). Otherwise,
  /// p.y must be within 0..256.
  ///
  /// WARNING: This will not send any packets to players! This function is meant
  /// for use by the world directly, or during use terrain generation. If you
  /// call this function without sending any updates yourself, no one in render
  /// distance will see any of these changes!
  pub fn set_type(&mut self, p: RelPos, ty: block::Type) -> Result<(), PosError> {
    let p = self.transform_pos(p)?;
    self.inner.set_type_id(p, ty.id(), ty.kind(), &self.wm.block_behaviors()).unwrap();
    Ok(())
  }

  pub fn set_type_with_conv(
    &mut self,
    p: RelPos,
    f: impl FnOnce(&block::TypeConverter) -> block::Type,
  ) -> Result<(), PosError> {
    let p = self.transform_pos(p)?;
    let ty = f(self.wm.block_converter());
    self.inner.set_type_id(p, ty.id(), ty.kind(), &self.wm.block_behaviors()).unwrap();
    Ok(())
  }

  /// Sets a block within this chunk. This is the same as
  /// [`set_type`](Self::set_type), but it uses a kind instead of a type. This
  /// will use the default type of the given kind. For example, an oak log would
  /// be placed facing upwards (along the Y axis), as this is the default type
  /// for that block.
  ///
  /// WARNING: This will not send any packets to players! This function is meant
  /// for use by the world directly, or during use terrain generation. If you
  /// call this function without sending any updates yourself, no one in render
  /// distance will see any of these changes!
  pub fn set_kind(&mut self, p: RelPos, kind: block::Kind) -> Result<(), PosError> {
    let p = self.transform_pos(p)?;
    let ty = self.wm.block_converter().get(kind).default_type();
    self.inner.set_type_id(p, ty.id(), ty.kind(), &self.wm.block_behaviors()).unwrap();
    Ok(())
  }

  /// Fills the region within this chunk. Min and max must be within the chunk
  /// column (see [`set_type`](Self::set_type)), and min must be less than or
  /// equal to max.
  ///
  /// Since multi chunks always store information in a paletted chunk, this will
  /// always be faster than calling [`set_type`](Self::set_type) repeatedly.
  ///
  /// WARNING: This will not send any packets to players! This function is meant
  /// for use by the world directly, or during use terrain generation. If you
  /// call this function without sending any updates yourself, no one in render
  /// distance will see any of these changes!
  pub fn fill(&mut self, min: RelPos, max: RelPos, ty: block::Type) -> Result<(), PosError> {
    let min = self.transform_pos(min)?;
    let max = self.transform_pos(max)?;
    self.inner.inner.fill(min, max, ty.id()).unwrap();
    // TODO: Update light correctly.
    self.inner.update_light(min);
    self.inner.update_light(max);
    Ok(())
  }

  /// This is the same as [`fill`](Self::fill), but it converts the block kind
  /// to it's default type.
  ///
  /// WARNING: This will not send any packets to players! This function is meant
  /// for use by the world directly, or during use terrain generation. If you
  /// call this function without sending any updates yourself, no one in render
  /// distance will see any of these changes!
  pub fn fill_kind(&mut self, min: RelPos, max: RelPos, kind: block::Kind) -> Result<(), PosError> {
    let min = self.transform_pos(min)?;
    let max = self.transform_pos(max)?;
    self
      .inner
      .inner
      .fill(min, max, self.wm.block_converter().get(kind).default_type().id())
      .unwrap();
    // TODO: Update light correctly.
    self.inner.update_light(min);
    self.inner.update_light(max);
    Ok(())
  }

  /// Gets the type of a block within this chunk. `p` must be within the chunk.
  /// See [`set_kind`](Self::set_kind) for more.
  ///
  /// This returns a specific block type. If you only need to block kind, prefer
  /// [`get_kind`](Self::get_kind).
  pub fn get_type(&self, p: RelPos) -> Result<block::Type, PosError> {
    let p = self.transform_pos(p)?;
    Ok(
      self
        .wm
        .block_converter()
        .type_from_id(self.inner.inner.get_block(p).unwrap(), BlockVersion::latest()),
    )
  }

  /// Gets the type of a block within this chunk. Pos must be within the chunk.
  /// See [`set_kind`](Self::set_kind) for more.
  pub fn get_kind(&self, p: RelPos) -> Result<block::Kind, PosError> {
    let p = self.transform_pos(p)?;
    Ok(
      self
        .wm
        .block_converter()
        .kind_from_id(self.inner.inner.get_block(p).unwrap(), BlockVersion::latest()),
    )
  }

  pub fn tes(&self) -> &HashMap<RelPos, Arc<dyn TileEntity>> { &self.inner.tes }
  pub(crate) fn tes_mut(&mut self) -> &mut HashMap<RelPos, Arc<dyn TileEntity>> {
    &mut self.inner.tes
  }

  pub fn get_te(&self, p: RelPos) -> Result<Option<Arc<dyn TileEntity>>, PosError> {
    let p = self.transform_pos(p)?;
    Ok(self.inner.tes.get(&p).cloned())
  }

  /// Transforms the given position to be used directly in a `Chunk`. This is
  /// because a `Chunk` cannot accept positions with a negative Y value, but
  /// worlds can have negative block positions.
  pub fn transform_pos(&self, mut p: RelPos) -> Result<RelPos, PosError> {
    if p.y() < self.inner.min_y || p.y() >= self.inner.height as i32 - self.inner.min_y {
      Err(p.err("is outside the world".into()))
    } else {
      p = p.add_y(self.inner.min_y);
      Ok(p)
    }
  }

  /// Returns the inner paletted chunk in this MultiChunk. This can be used to
  /// access the block data directly. All ids are the latest version block
  /// states.
  ///
  /// Before accessing the chunk, [`transform_pos`](Self::transform_pos) should
  /// be used to translate the blocks to the correct coordinates.
  ///
  /// Note that the returned chunk only validates the positions are positive. It
  /// will allocate all the space it needs to place a block at whatever `Y`
  /// value you specify.
  pub fn inner(&self) -> &Chunk<PalettedSection> { &self.inner.inner }
  /// Same as [`inner`](Self::inner), but returns a mutable reference.
  pub fn inner_mut(&mut self) -> &mut Chunk<PalettedSection> { &mut self.inner.inner }

  /// Returns a reference to the global type converter. Used to convert a block
  /// id to/from any version.
  pub fn type_converter(&self) -> &Arc<block::TypeConverter> { self.wm.block_converter() }

  /// Returns the sky light information for this chunk. Used to send lighting
  /// data to clients.
  pub fn sky_light(&self) -> &Option<LightChunk<SkyLight>> { &self.inner.sky }
  /// Returns the block light information for this chunk. Used to send lighting
  /// data to clients.
  pub fn block_light(&self) -> &LightChunk<BlockLight> { &self.inner.block }

  /// Will enable/disable lighting. Chunks have lighting enabled by default. If
  /// enabled, and if it was previously disabled, all the lighting information
  /// will be recalculated (which is very slow).
  pub fn enable_lighting(&mut self, enabled: bool) { self.inner.enable_lighting(enabled) }
}

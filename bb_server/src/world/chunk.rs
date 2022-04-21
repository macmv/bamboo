use crate::block;
use bb_common::{
  chunk::{paletted::Section as PalettedSection, BlockLight, Chunk, LightChunk, SkyLight},
  math::{Pos, PosError},
  version::BlockVersion,
};
use parking_lot::Mutex;
use std::sync::{atomic::AtomicU32, Arc};

/// A chunk in the world with a number of people viewing it. If the count is at
/// 0, then this chunk is essentially flagged for unloading. Chunks are unloaded
/// lazily, so this chunk will just end up being cleaned up in the future.
pub struct CountedChunk {
  pub(super) count: AtomicU32,
  pub chunk:        Mutex<MultiChunk>,
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
  inner:        Chunk<PalettedSection>,
  sky:          Option<LightChunk<SkyLight>>,
  block:        LightChunk<BlockLight>,
  types:        Arc<block::TypeConverter>,
  /// Set to false when the world is generating, which makes things much faster.
  update_light: bool,
}

impl CountedChunk {
  /// Creates a new counted chunk with the counter at 0.
  pub fn new(c: MultiChunk) -> CountedChunk {
    CountedChunk { count: 0.into(), chunk: Mutex::new(c) }
  }
}

impl MultiChunk {
  /// Creates an empty chunk.
  ///
  /// The second argument is for sky light data. Places like the nether do not
  /// contain sky light information, so the sky light data is not present.
  pub fn new(types: Arc<block::TypeConverter>, sky: bool) -> MultiChunk {
    MultiChunk {
      inner: Chunk::new(15),
      sky: if sky { Some(LightChunk::new()) } else { None },
      block: LightChunk::new(),
      types,
      update_light: true,
    }
  }

  /// Sets a block within this chunk. `p.x` and `p.z` must be within 0..16. If
  /// the server supports multi-height worlds (not implemented yet), then p.y
  /// needs to be within the world height (whatever that may be). Otherwise,
  /// p.y must be within 0..256.
  ///
  /// WARNING: This will not send any packets to players! This function is meant
  /// for use by the world directly, or during use terrain generation. If you
  /// call this function without sending any updates yourself, no one in render
  /// distance will see any of these changes!
  pub fn set_type(&mut self, p: Pos, ty: block::Type) -> Result<(), PosError> {
    self.inner.set_block(p, ty.id())?;
    self.update_light(p);
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
  pub fn set_kind(&mut self, p: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.inner.set_block(p, self.types.get(kind).default_type().id())?;
    self.update_light(p);
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
  pub fn fill(&mut self, min: Pos, max: Pos, ty: block::Type) -> Result<(), PosError> {
    self.inner.fill(min, max, ty.id())?;
    // TODO: Update light correctly.
    self.update_light(min);
    self.update_light(max);
    Ok(())
  }

  /// This is the same as [`fill`](Self::fill), but it converts the block kind
  /// to it's default type.
  ///
  /// WARNING: This will not send any packets to players! This function is meant
  /// for use by the world directly, or during use terrain generation. If you
  /// call this function without sending any updates yourself, no one in render
  /// distance will see any of these changes!
  pub fn fill_kind(&mut self, min: Pos, max: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.inner.fill(min, max, self.types.get(kind).default_type().id())?;
    Ok(())
  }

  /// Gets the type of a block within this chunk. `p` must be within the chunk.
  /// See [`set_kind`](Self::set_kind) for more.
  ///
  /// This returns a specific block type. If you only need to block kind, prefer
  /// [`get_kind`](Self::get_kind).
  pub fn get_type(&self, p: Pos) -> Result<block::Type, PosError> {
    Ok(self.types.type_from_id(self.inner.get_block(p)?, BlockVersion::latest()))
  }

  /// Gets the type of a block within this chunk. Pos must be within the chunk.
  /// See [`set_kind`](Self::set_kind) for more.
  pub fn get_kind(&self, p: Pos) -> Result<block::Kind, PosError> {
    Ok(self.types.kind_from_id(self.inner.get_block(p)?, BlockVersion::latest()))
  }

  /// Returns the inner paletted chunk in this MultiChunk. This can be used to
  /// access the block data directly. All ids are the latest version block
  /// states.
  pub fn inner(&self) -> &Chunk<PalettedSection> { &self.inner }
  /// Same as [`inner`](Self::inner), but returns a mutable reference.
  pub fn inner_mut(&mut self) -> &mut Chunk<PalettedSection> { &mut self.inner }

  /// Returns a reference to the global type converter. Used to convert a block
  /// id to/from any version.
  pub fn type_converter(&self) -> &block::TypeConverter { &self.types }

  /// Returns the sky light information for this chunk. Used to send lighting
  /// data to clients.
  pub fn sky_light(&self) -> &Option<LightChunk<SkyLight>> { &self.sky }
  /// Returns the block light information for this chunk. Used to send lighting
  /// data to clients.
  pub fn block_light(&self) -> &LightChunk<BlockLight> { &self.block }

  /// Will enable/disable lighting. Chunks have lighting enabled by default. If
  /// enabled, and if it was previously disabled, all the lighting information
  /// will be recalculated (which is very slow).
  pub fn enable_lighting(&mut self, enabled: bool) {
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
  fn update_light(&mut self, pos: Pos) {
    if self.update_light {
      if let Some(sky) = &mut self.sky {
        sky.update(&self.inner, pos);
      }
      self.block.update(&self.inner, pos);
    }
  }
}

use std::sync::Arc;

use sc_common::{
  chunk::{paletted::Section as PalettedSection, BlockLightChunk, Chunk, SkyLightChunk},
  math::{Pos, PosError},
  version::BlockVersion,
};

use crate::block;

pub struct MultiChunk {
  inner: Chunk<PalettedSection>,
  sky:   Option<SkyLightChunk>,
  block: BlockLightChunk,
  types: Arc<block::TypeConverter>,
}

impl MultiChunk {
  /// Creates an empty chunk. Currently, it just creates a seperate chunk for
  /// every supported version. In the future, it will take a list of versions as
  /// parameters. If it is fast enough, I might generate a mapping of all new
  /// block ids and how they can be transformed into old block ids. Then, this
  /// would only store one chunk, and would perform all conversions when you
  /// actually tried to get an old id.
  ///
  /// The second argument is for sky light data. Places like the nether do not
  /// contain sky light information, so the sky light data is not present.
  pub fn new(types: Arc<block::TypeConverter>, sky: bool) -> MultiChunk {
    MultiChunk {
      inner: Chunk::new(),
      sky: if sky { Some(SkyLightChunk::new()) } else { None },
      block: BlockLightChunk::new(),
      types,
    }
  }

  /// Sets a block within this chunk. p.x and p.z must be within 0..16. If the
  /// server is only running on 1.17, then p.y needs to be within the world
  /// height (whatever that may be). Otherwise, p.y must be within 0..256.
  pub fn set_type(&mut self, p: Pos, ty: block::Type) -> Result<(), PosError> {
    self.inner.set_block(p, ty.id())?;
    self.update_light(p);
    Ok(())
  }

  /// Sets a block within this chunk. This is the same as
  /// [`set_type`](Self::set_type), but it uses a kind instead of a type. This
  /// will use the default type of the given kind.
  pub fn set_kind(&mut self, p: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.inner.set_block(p, self.types.get(kind).default_type().id())?;
    self.update_light(p);
    Ok(())
  }

  /// Fills the region within this chunk. Min and max must be within the chunk
  /// column (see [`set_type`](Self::set_type)), and min must be less than or
  /// equal to max.
  ///
  /// Since multi chunks always store a fixed chunk and a paletted chunk, this
  /// will always be faster than calling set_type in a loop.
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

  /// Gets the type of a block within this chunk. Pos must be within the chunk.
  /// See [`set_kind`](Self::set_kind) for more.
  ///
  /// This returns a specific block type. If you only need to block kind, prefer
  /// [`get_kind`](Self::get_kind)
  pub fn get_type(&self, p: Pos) -> Result<block::Type, PosError> {
    Ok(self.types.type_from_id(self.inner.get_block(p)?, BlockVersion::V1_16))
  }

  /// Gets the type of a block within this chunk. Pos must be within the chunk.
  /// See [`set_block`](Self::set_block) for more.
  pub fn get_kind(&self, p: Pos) -> Result<block::Kind, PosError> {
    Ok(self.types.kind_from_id(self.inner.get_block(p)?, BlockVersion::V1_16))
  }

  /// Builds a heightmap of this chunk. Each long contains 9 bit entries, where
  /// each entry is the height of the world at the given X, Z coordinate. This
  /// is used within 1.14+ protocol data, and is a needlessly complicated format
  /// that you shouldn't waste any time thinking about.
  ///
  /// The only reason these are signed is because of NBT long arrays. In
  /// reality, they should be read as unsigned longs.
  pub fn build_heightmap(&self) -> Vec<i64> {
    let mut heightmap = vec![0; 256 * 9 / 64];
    let mut shift = 0;
    let mut index = 0;
    for _ in 0..16 {
      for _ in 0..16 {
        // TODO: Set this to the height at the given coordinate, not just 0
        let v = 0_u64;
        if shift > 64 - 9 {
          heightmap[index] |= (v.overflowing_shl(shift).0 & 0b111111111 << (64 - 9)) as i64;
          heightmap[index + 1] |= (v >> (64 - shift)) as i64;
        } else {
          heightmap[index] |= (v.overflowing_shl(shift).0) as i64;
        }
        shift += 9;
        if shift > 64 {
          shift -= 64;
          index += 1;
        }
      }
    }
    heightmap
  }

  /// Returns the inner paletted chunk in this MultiChunk. This can be used to
  /// access the block data directly. All ids are the latest version block
  /// states.
  pub fn inner(&self) -> &Chunk<PalettedSection> { &self.inner }

  /// Returns a reference to the global type converter. Used to convert a block
  /// id to/from any version.
  pub fn type_converter(&self) -> &block::TypeConverter { &self.types }

  /// Returns the sky light information for this chunk. Used to send lighting
  /// data to clients.
  pub fn sky_light(&self) -> &Option<SkyLightChunk> { &self.sky }
  /// Returns the block light information for this chunk. Used to send lighting
  /// data to clients.
  pub fn block_light(&self) -> &BlockLightChunk { &self.block }

  fn update_light(&mut self, pos: Pos) {
    if let Some(sky) = &mut self.sky {
      sky.update(&self.inner, pos);
    }
    self.block.update(&self.inner, pos);
  }
}

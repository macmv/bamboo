use crate::gnet::cb::Packet;
use bb_common::{
  chunk::{paletted::Section, BlockLight, LightChunk, SkyLight},
  math::ChunkPos,
  version::{BlockVersion, ProtocolVersion},
};
use smallvec::SmallVec;

mod v1_14;
mod v1_15;
mod v1_16;
mod v1_17;
mod v1_18;
mod v1_8;
mod v1_9;

mod cb;
mod conv;
mod metadata;
mod sb;

pub use cb::{ToTcp, WriteError};
pub use conv::TypeConverter;
pub use sb::FromTcp;

pub use metadata::metadata;

pub struct ChunkWithPos {
  pos:         ChunkPos,
  full:        bool,
  sections:    Vec<Option<Section>>,
  sky_light:   Option<LightChunk<SkyLight>>,
  block_light: LightChunk<BlockLight>,
}

pub fn chunk(
  chunk: ChunkWithPos,
  ver: ProtocolVersion,
  conv: &TypeConverter,
) -> SmallVec<[Packet; 2]> {
  smallvec![match ver.block() {
    BlockVersion::V1_8 => v1_8::chunk(chunk, conv),
    BlockVersion::V1_9 | BlockVersion::V1_12 => v1_9::chunk(chunk, ver, conv),
    // ProtocolVersion::V1_13 => v1_13::serialize_chunk(pos, bit_map, &sections, conv),
    BlockVersion::V1_14 => v1_14::chunk(chunk, conv),
    BlockVersion::V1_15 => v1_15::chunk(chunk, conv),
    BlockVersion::V1_16 => v1_16::chunk(chunk, conv),
    BlockVersion::V1_17 => v1_17::chunk(chunk, conv),
    BlockVersion::V1_18 => v1_18::chunk(chunk, conv),
    _ => todo!("chunk on version {}", ver),
  }]
}

pub fn multi_block_change(
  pos: ChunkPos,
  y: i32,
  changes: Vec<u64>,
  ver: ProtocolVersion,
  conv: &TypeConverter,
) -> Packet {
  match ver.block() {
    BlockVersion::V1_8 | BlockVersion::V1_9 | BlockVersion::V1_12 => {
      v1_8::multi_block_change(pos, y, changes, ver, conv)
    }
    // ProtocolVersion::V1_13 => v1_13::serialize_chunk(pos, bit_map, &sections, conv),
    // BlockVersion::V1_14 => v1_14::chunk(pos, full, bit_map, &sections, conv),
    // ProtocolVersion::V1_15 => v1_15::serialize_chunk(pos, c),
    // ProtocolVersion::V1_16 => v1_16::serialize_chunk(pos, c),
    _ => todo!("multi block change on version {}", ver),
  }
}

impl ChunkWithPos {
  /// Generates the bitmap used on versions 1.8-1.17 for chunk sections. 1.18+
  /// sends all chunks, with a special "empty section" format, which only takes
  /// a few bytes.
  pub fn old_bit_map(&self) -> u16 {
    let mut bit_map = 0;
    for (y, section) in self.sections.iter().enumerate() {
      if section.is_some() {
        bit_map |= 1 << y;
      }
    }
    bit_map
  }
}

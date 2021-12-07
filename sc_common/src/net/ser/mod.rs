use crate::{
  chunk::paletted::Section,
  math::ChunkPos,
  net::VersionConverter,
  version::{BlockVersion, ProtocolVersion},
};
use sc_generated::net::cb::Packet;

mod v1_8;
mod v1_9;

pub fn chunk(
  pos: ChunkPos,
  bit_map: u16,
  sections: Vec<Section>,
  ver: ProtocolVersion,
  conv: &impl VersionConverter,
) -> Packet {
  match ver.block() {
    BlockVersion::V1_8 => v1_8::chunk(pos, bit_map, &sections, conv),
    BlockVersion::V1_9 | BlockVersion::V1_12 => {
      v1_9::chunk(pos, bit_map, &sections, ver.block(), conv)
    }
    // ProtocolVersion::V1_13 => v1_13::serialize_chunk(pos, c),
    // ProtocolVersion::V1_14 => v1_14::serialize_chunk(pos, c),
    // ProtocolVersion::V1_15 => v1_15::serialize_chunk(pos, c),
    // ProtocolVersion::V1_16 => v1_16::serialize_chunk(pos, c),
    _ => todo!("chunk on version {}", ver),
  }
}

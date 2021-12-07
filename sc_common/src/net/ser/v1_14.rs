use crate::{
  chunk::paletted::Section,
  math::ChunkPos,
  net::VersionConverter,
  util::{
    nbt::{Tag, NBT},
    Buffer,
  },
};
use sc_generated::{net::cb::Packet, version::BlockVersion};

// CHANGES (since 1.12.2):
// No length is written for >8 bpb
// Biome is i32, not u8
// Added the MOTION_BLOCKING field. This is a heightmap, stored in NBT.
// Added a u16 for non air blocks at the start of each section.
// Moved lighting data into another packet, so it is no longer included.
pub fn chunk(
  pos: ChunkPos,
  bit_map: u16,
  sections: &[Section],
  conv: &impl VersionConverter,
) -> Packet {
  let biomes = true; // Always true with new chunk set
  let _skylight = true; // Assume overworld

  let mut chunk_data = Buffer::new(vec![]);
  // Iterates through chunks in order, from ground up. Flatten removes None
  // sections.
  for s in sections {
    chunk_data.write_u16(s.non_air_blocks() as u16);
    chunk_data.write_u8(s.data().bpe() as u8);
    if s.data().bpe() <= 8 {
      chunk_data.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_data.write_varint(conv.block_to_old(*g as u32, BlockVersion::V1_14) as i32);
      }
    }
    let longs = s.data().long_array();
    chunk_data.write_varint(longs.len() as i32);
    chunk_data.write_buf(&longs.iter().map(|v| v.to_be_bytes()).flatten().collect::<Vec<u8>>());
  }

  if biomes {
    for _ in 0..256 {
      chunk_data.write_i32(127); // Void biome
    }
  }

  let heightmap = vec![];
  let heightmap = NBT::new("", Tag::compound(&[("MOTION_BLOCKING", Tag::LongArray(heightmap))]));

  let mut data = Buffer::new(Vec::with_capacity(chunk_data.len()));
  data.write_buf(&heightmap.serialize());
  data.write_varint(chunk_data.len() as i32);
  data.write_buf(&chunk_data.into_inner());
  data.write_varint(0); // No block entities
  Packet::ChunkDataV14 {
    chunk_x:                pos.x(),
    chunk_z:                pos.z(),
    is_full_chunk:          true,
    vertical_strip_bitmask: bit_map.into(),
    heightmaps:             None,
    data:                   vec![],
    block_entities:         None,
    unknown:                data.into_inner(),
  }
}

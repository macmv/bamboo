use super::TypeConverter;
use sc_common::{
  chunk::paletted::Section,
  gnet::cb::Packet,
  math::ChunkPos,
  util::{
    nbt::{Tag, NBT},
    Buffer,
  },
  version::BlockVersion,
};

// CHANGES:
// Biomes are now a length prefixed varint array, instead of an int array.
pub fn chunk(
  pos: ChunkPos,
  full: bool,
  bit_map: u16,
  sections: &[Section],
  conv: &TypeConverter,
) -> Packet {
  let biomes = full;
  let _skylight = true; // Assume overworld

  let mut chunk_data = Buffer::new(vec![]);
  for s in sections {
    chunk_data.write_u16(s.non_air_blocks() as u16);
    chunk_data.write_u8(s.data().bpe() as u8);
    if s.data().bpe() <= 8 {
      chunk_data.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_data.write_varint(conv.block_to_old(*g as u32, BlockVersion::V1_16) as i32);
      }
    }
    let longs = s.data().long_array();
    chunk_data.write_varint(longs.len() as i32);
    chunk_data.write_buf(&longs.iter().map(|v| v.to_be_bytes()).flatten().collect::<Vec<u8>>());
  }

  let mut biome_data = Buffer::new(vec![]);
  if biomes {
    biome_data.write_varint(1024); // Length of biomes
    for _ in 0..1024 {
      // The first biome in the list of biomes in the dimension codec
      biome_data.write_varint(0);
    }
  }

  let heightmap = vec![];
  let heightmap = NBT::new("", Tag::compound(&[("MOTION_BLOCKING", Tag::LongArray(heightmap))]));

  let mut data = Buffer::new(Vec::with_capacity(chunk_data.len()));
  data.write_buf(&heightmap.serialize());
  data.write_buf(&biome_data.into_inner());
  data.write_varint(chunk_data.len() as i32);
  data.write_buf(&chunk_data.into_inner());
  data.write_varint(0); // No block entities
  Packet::ChunkDataV14 {
    chunk_x:                pos.x(),
    chunk_z:                pos.z(),
    is_full_chunk:          full,
    vertical_strip_bitmask: bit_map.into(),
    heightmaps:             None,
    data:                   vec![],
    block_entities:         None,
    unknown:                data.into_inner(),
  }
}

use super::TypeConverter;
use crate::gnet::cb::Packet;
use sc_common::{
  chunk::{paletted::Section, BlockLight, LightChunk, SkyLight},
  math::ChunkPos,
  util::{
    nbt::{Tag, NBT},
    Buffer,
  },
  version::BlockVersion,
};

// CHANGES:
// - Bitmask was removed, and we now need to send empty sections.
// - Biome array now uses paletted section format, and is part of each chunk
//   section (before it was part of the chunk column).
// - Light update packet was merged into this packet.
pub fn chunk(
  pos: ChunkPos,
  full: bool,
  bit_map: u16,
  sections: &[Section],
  sky_light: Option<LightChunk<SkyLight>>,
  block_light: LightChunk<BlockLight>,
  conv: &TypeConverter,
) -> Packet {
  let biomes = full;
  let _skylight = true; // Assume overworld

  let mut chunk_data = Buffer::new(vec![]);

  // This is the length in longs that the bit map takes up.
  // chunk_data.write_varint(1);
  // chunk_data.write_u64(bit_map.into());

  // 1.18 requires all chunk sections to be sent
  let mut idx = 0;
  for i in 0..16 {
    if bit_map & (1 << i) == 0 {
      chunk_data.write_u16(0); // No non air blocks

      // Paletted container for chunk data
      chunk_data.write_u8(0); // 0 bpe
      chunk_data.write_varint(0); // our one value is 0
      chunk_data.write_varint(0); // no data

      // Paletted container for biome data
      chunk_data.write_u8(0); // 0 bpe
      chunk_data.write_varint(0); // our one value is 0
      chunk_data.write_varint(0); // no data
      continue;
    }
    let s = &sections[idx];
    chunk_data.write_u16(s.non_air_blocks() as u16);

    // Paletted container for chunk data
    chunk_data.write_u8(s.data().bpe() as u8);
    if s.data().bpe() <= 8 {
      chunk_data.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_data.write_varint(conv.block_to_old(*g as u32, BlockVersion::V1_18) as i32);
      }
    }
    let longs = s.data().long_array();
    chunk_data.write_varint(longs.len() as i32);
    chunk_data.write_buf(&longs.iter().map(|v| v.to_be_bytes()).flatten().collect::<Vec<u8>>());

    // Paletted container for biome data
    if biomes {
      /*
      let len = 64; // 64 entries
      let num_longs = len / (64 / 4);
      chunk_data.write_u8(4); // 4 bits per entry
      chunk_data.write_varint(1); // Palette has 1 entry
      chunk_data.write_varint(0); // The entry is `0`

      chunk_data.write_varint(num_longs); // Number of longs in the following array.
      for _ in 0..num_longs {
        // Every 4 bits refers to a `0` in the palette, so we can just write it all as
        // zeros.
        chunk_data.write_u64(0);
      }
      */
      // New special 'single value' palette
      chunk_data.write_u8(0); // 0 bits per entry
      chunk_data.write_varint(0); // The single entry is `0`
      chunk_data.write_varint(0); // The data length is `0`

      // No data follows, as this signifies that the entire section is just that
      // one biome.
    }
    idx += 1;
  }

  let heightmap = vec![];
  let heightmap = NBT::new("", Tag::compound(&[("MOTION_BLOCKING", Tag::LongArray(heightmap))]));

  let mut data = Buffer::new(Vec::with_capacity(chunk_data.len()));

  data.write_buf(&heightmap.serialize());

  data.write_varint(chunk_data.len() as i32);
  data.write_buf(&chunk_data.into_inner());
  data.write_varint(0); // No block entities

  // Light update stuff
  data.write_bool(true); // Client should trust edges

  let mut sky_bitmap: u64 = 0;
  let mut sky_len = 0;
  if let Some(sky) = &sky_light {
    for (i, s) in sky.sections().iter().enumerate() {
      if s.is_some() {
        sky_bitmap |= 1 << i as u64;
        sky_len += 2048;
      }
    }
  }
  let mut block_bitmap: u64 = 0;
  let mut block_len = 0;
  for (i, s) in block_light.sections().iter().enumerate() {
    if s.is_some() {
      block_bitmap |= 1 << i as u64;
      block_len += 2048;
    }
  }

  // Sky light bitset
  data.write_varint(1);
  data.write_u64(sky_bitmap);
  // Block light bitset
  data.write_varint(1);
  data.write_u64(block_bitmap);
  // Empty sky light bitset
  data.write_varint(1);
  data.write_u64(0x0);
  // Empty block light bitset
  data.write_varint(1);
  data.write_u64(0x0);
  // Sky light length
  data.write_varint(sky_len);
  if let Some(sky) = sky_light {
    for s in sky.sections() {
      if let Some(s) = s {
        data.write_buf(s.data());
      }
    }
  }
  // Block light length
  data.write_varint(block_len);
  for s in block_light.sections() {
    if let Some(s) = s {
      data.write_buf(s.data());
    }
  }

  Packet::ChunkDataV17 {
    chunk_x:                pos.x(),
    chunk_z:                pos.z(),
    max_data_length:        None,
    vertical_strip_bitmask: None,
    heightmaps:             None,
    data:                   vec![],
    biome_array:            vec![],
    block_entities:         None,
    unknown:                data.into_inner(),
  }
}

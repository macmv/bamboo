use super::{ChunkWithPos, TypeConverter};
use crate::gnet::cb::{packet, Packet};
use bb_common::{
  chunk::Chunk,
  nbt::{Tag, NBT},
  util::Buffer,
  version::ProtocolVersion,
};

// CHANGES:
// - Boolean for edge of chunk was removed.
pub fn chunk(chunk: ChunkWithPos, ver: ProtocolVersion, conv: &TypeConverter) -> Packet {
  let biomes = chunk.full;
  let _skylight = true; // Assume overworld

  let mut chunk_data = vec![];
  let mut chunk_buf = Buffer::new(&mut chunk_data);

  // This is the length in longs that the bit map takes up.
  // chunk_data.write_varint(1);
  // chunk_data.write_u64(bit_map.into());

  // 1.18 requires all chunk sections to be sent
  for s in &chunk.sections {
    if let Some(s) = s {
      chunk_buf.write_u16(s.non_air_blocks() as u16);

      // Paletted container for chunk data
      chunk_buf.write_u8(s.data().bpe());
      if s.data().bpe() <= 8 {
        chunk_buf.write_varint(s.palette().len() as i32);
        for g in s.palette() {
          chunk_buf.write_varint(conv.block_to_old(*g, ver.block()) as i32);
        }
      }
      let longs = s.data().long_array();
      chunk_buf.write_varint(longs.len() as i32);
      longs.iter().for_each(|v| chunk_buf.write_buf(&v.to_be_bytes()));

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
        chunk_buf.write_u8(0); // 0 bits per entry
        chunk_buf.write_varint(0); // The single entry is `0`
        chunk_buf.write_varint(0); // The data length is `0`

        // No data follows, as this signifies that the entire section is just
        // that one biome.
      }
    } else {
      chunk_buf.write_u16(0); // No non air blocks

      // Paletted container for chunk data
      chunk_buf.write_u8(0); // 0 bpe
      chunk_buf.write_varint(0); // our one value is 0
      chunk_buf.write_varint(0); // no data

      // Paletted container for biome data
      chunk_buf.write_u8(0); // 0 bpe
      chunk_buf.write_varint(0); // our one value is 0
      chunk_buf.write_varint(0); // no data
    }
  }

  let c = Chunk::from_sections(chunk.packet.sections, 15);
  let heightmap = c.build_heightmap_new();
  let heightmap =
    NBT::new("", Tag::new_compound(&[("MOTION_BLOCKING", Tag::LongArray(heightmap))]));

  let mut data = Vec::with_capacity(chunk_buf.len());
  let mut buf = Buffer::new(&mut data);

  buf.write_buf(&heightmap.serialize());

  buf.write_varint(chunk_buf.len() as i32);
  buf.write_buf(&chunk_data);
  buf.write_varint(0); // No block entities

  let mut sky_bitmap: u64 = 0;
  let mut sky_empty_bitmap: u64 = 0;
  let mut sky_len = 0;
  for y in 0..16 {
    if let Some(sky) = &chunk.packet.sky_light {
      if sky.get_section_opt(y).is_some() {
        sky_bitmap |= 1 << y as u64;
        sky_len += 1;
        continue;
      }
    }
    sky_empty_bitmap |= 1 << y as u64;
  }
  let mut block_bitmap: u64 = 0;
  let mut block_empty_bitmap: u64 = 0;
  let mut block_len = 0;
  for y in 0..16 {
    if chunk.packet.block_light.get_section_opt(y).is_some() {
      block_bitmap |= 1 << y as u64;
      block_len += 1;
    } else {
      block_empty_bitmap |= 1 << y as u64;
    }
  }

  sky_bitmap <<= 1;
  sky_empty_bitmap <<= 1;
  sky_empty_bitmap |= 1 | (1 << 17);
  block_bitmap <<= 1;
  block_empty_bitmap <<= 1;
  block_empty_bitmap |= 1 | (1 << 17);

  // Sky light bitset
  buf.write_varint(1);
  buf.write_u64(sky_bitmap);
  // Block light bitset
  buf.write_varint(1);
  buf.write_u64(block_bitmap);
  // Empty sky light bitset
  buf.write_varint(1);
  buf.write_u64(sky_empty_bitmap);
  // Empty block light bitset
  buf.write_varint(1);
  buf.write_u64(block_empty_bitmap);
  // Sky light length
  buf.write_varint(sky_len);
  if let Some(sky) = chunk.packet.sky_light {
    for s in sky.sections().iter().flatten() {
      buf.write_varint(s.data().len() as i32);
      buf.write_buf(s.data());
    }
  }
  // Block light length
  buf.write_varint(block_len);
  for s in chunk.packet.block_light.sections().iter().flatten() {
    buf.write_varint(s.data().len() as i32);
    buf.write_buf(s.data());
  }

  packet::ChunkDataV17 {
    chunk_x: chunk.packet.pos.x(),
    chunk_z: chunk.packet.pos.z(),
    unknown: data,
  }
  .into()
}

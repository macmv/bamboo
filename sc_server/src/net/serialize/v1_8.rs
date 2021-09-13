use crate::world::chunk::MultiChunk;
use sc_common::{math::ChunkPos, net::cb, util::Buffer};
use std::convert::TryInto;

pub fn serialize_chunk(pos: ChunkPos, c: &MultiChunk) -> cb::Packet {
  let mut bit_map = 0;
  let c = c.get_fixed();

  let skylight = true;
  let biomes = true;

  let mut total_sections = 0;
  for s in c.sections().into_iter() {
    if s.is_some() {
      total_sections += 1;
    }
  }

  let data_len = total_sections * 16 * 16 * 16 * 2 // Chunk data
    + (total_sections * 16 * 16 * 16 / 2) // Block light
    + if skylight { total_sections * 16 * 16 * 16 / 2 } else { 0 } // Sky light
    + if biomes { 256 } else { 0 }; // Biomes

  // The most it will be is data_len + max varint len
  let mut chunk_data = Buffer::new(Vec::with_capacity(data_len + 5));
  chunk_data.write_varint(data_len.try_into().unwrap());
  let prefix_len = chunk_data.len();

  for (y, s) in c.sections().into_iter().enumerate() {
    if let Some(s) = s {
      let s = s.unwrap_fixed();
      bit_map |= 1 << y;
      // These are little endian. I'm assuming that the person who wrote the
      // networking for Minecraft and the person who wrote chunk decoding were not
      // consistent.
      chunk_data
        .write_buf(&s.data().iter().map(|v| v.to_le_bytes()).flatten().collect::<Vec<u8>>());
    }
  }
  // Light data
  for _ in 0..total_sections * 16 * 16 * 16 / 2 {
    // Each lighting value is 1/2 byte
    chunk_data.write_u8(0xff);
  }
  if skylight {
    for _ in 0..total_sections * 16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      chunk_data.write_u8(0xff);
    }
  }
  if biomes {
    for _ in 0..256 {
      chunk_data.write_u8(127); // Void biome
    }
  }
  debug_assert_eq!(chunk_data.len() - prefix_len, data_len, "unexpected chunk data len");

  cb::Packet::MapChunk {
    x:                                     pos.x(),
    z:                                     pos.z(),
    ground_up:                             true,
    chunk_data:                            chunk_data.into_inner(),
    bit_map_v1_8:                          Some(bit_map),
    bit_map_v1_9:                          None,
    block_entities_v1_9_4:                 None,
    heightmaps_v1_14:                      None,
    biomes_v1_15:                          None,
    biomes_v1_16_2:                        None,
    ignore_old_data_v1_16_removed_v1_16_2: None,
  }
}

pub fn serialize_partial_chunk(pos: ChunkPos, c: &MultiChunk, min: u32, max: u32) -> cb::Packet {
  let mut bit_map = 0;
  let c = c.get_fixed();

  let skylight = true;
  let biomes = true;

  let total_sections = (max - min + 1) as usize;

  let data_len = total_sections * 16 * 16 * 16 * 2 // Chunk data
    + (total_sections * 16 * 16 * 16 / 2) // Block light
    + if skylight { total_sections * 16 * 16 * 16 / 2 } else { 0 } // Sky light
    + if biomes { 256 } else { 0 }; // Biomes

  // The most it will be is data_len + max varint len
  let mut chunk_data = Buffer::new(Vec::with_capacity(data_len + 5));
  chunk_data.write_varint(data_len.try_into().unwrap());
  let prefix_len = chunk_data.len();

  for (y, s) in c.sections().into_iter().enumerate() {
    if y >= min as usize && y <= max as usize {
      let s =
        s.as_ref().expect(&format!("the section at {} has not been loaded!", y)).unwrap_fixed();
      bit_map |= 1 << y;
      // These are little endian. I'm assuming that the person who wrote the
      // networking for Minecraft and the person who wrote chunk decoding were not
      // consistent.
      chunk_data
        .write_buf(&s.data().iter().map(|v| v.to_le_bytes()).flatten().collect::<Vec<u8>>());
    }
  }
  // Light data
  for _ in 0..total_sections * 16 * 16 * 16 / 2 {
    // Each lighting value is 1/2 byte
    chunk_data.write_u8(0xff);
  }
  if skylight {
    for _ in 0..total_sections * 16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      chunk_data.write_u8(0xff);
    }
  }
  if biomes {
    for _ in 0..256 {
      chunk_data.write_u8(127); // Void biome
    }
  }
  debug_assert_eq!(chunk_data.len() - prefix_len, data_len, "unexpected chunk data len");

  cb::Packet::MapChunk {
    x:                                     pos.x(),
    z:                                     pos.z(),
    ground_up:                             false, // Because partial chunks
    chunk_data:                            chunk_data.into_inner(),
    bit_map_v1_8:                          Some(bit_map),
    bit_map_v1_9:                          None,
    block_entities_v1_9_4:                 None,
    heightmaps_v1_14:                      None,
    biomes_v1_15:                          None,
    biomes_v1_16_2:                        None,
    ignore_old_data_v1_16_removed_v1_16_2: None,
  }
}

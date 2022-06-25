use super::{ChunkWithPos, TypeConverter};
use crate::gnet::cb::{packet, Packet};
use bb_common::{
  nbt::{Tag, NBT},
  util::Buffer,
  version::BlockVersion,
};

// CHANGES:
// Added biomes as a separate field, which is 1024 elements, instead of 256
// elements.
pub fn chunk(chunk: ChunkWithPos, conv: &TypeConverter) -> Packet {
  let biomes = chunk.full;
  let _skylight = true; // Assume overworld

  let mut chunk_data = vec![];
  let mut chunk_buf = Buffer::new(&mut chunk_data);
  for s in chunk.sections.iter().flatten() {
    chunk_buf.write_u16(s.non_air_blocks() as u16);
    chunk_buf.write_u8(s.data().bpe() as u8);
    if s.data().bpe() <= 8 {
      chunk_buf.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_buf.write_varint(conv.block_to_old(*g as u32, BlockVersion::V1_15) as i32);
      }
    }
    let longs = s.data().old_long_array();
    chunk_buf.write_varint(longs.len() as i32);
    chunk_buf.reserve(longs.len() * 8); // 8 bytes per long
    longs.iter().for_each(|v| chunk_buf.write_buf(&v.to_be_bytes()));
  }

  let mut biome_data = vec![];
  let mut biome_buf = Buffer::new(&mut biome_data);
  if biomes {
    for _ in 0..1024 {
      biome_buf.write_i32(127); // Void biome
    }
  }

  let heightmap = vec![];
  let heightmap = NBT::new("", Tag::compound(&[("MOTION_BLOCKING", Tag::LongArray(heightmap))]));

  let mut data = Vec::with_capacity(chunk_buf.len());
  let mut buf = Buffer::new(&mut data);
  buf.write_buf(&heightmap.serialize());
  buf.write_buf(&biome_data);
  buf.write_varint(chunk_buf.len() as i32);
  buf.write_buf(&chunk_data);
  buf.write_varint(0); // No block entities

  packet::ChunkDataV14 {
    chunk_x:                chunk.pos.x(),
    chunk_z:                chunk.pos.z(),
    is_full_chunk:          chunk.full,
    vertical_strip_bitmask: chunk.old_bit_map().into(),
    unknown:                data,
  }
  .into()
}

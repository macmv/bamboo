use super::{ChunkWithPos, TypeConverter};
use crate::gnet::cb::{packet, Packet};
use bb_common::{util::Buffer, version::ProtocolVersion};

// Applies to 1.9 - 1.12, but 1.10 doesn't work, so idk
pub fn chunk(chunk: ChunkWithPos, ver: ProtocolVersion, conv: &TypeConverter) -> Packet {
  let biomes = chunk.full;
  let skylight = true; // Assume overworld

  let mut base = 16 * 16 / 2;
  let mut chunk_data = Vec::with_capacity(
    1024 + chunk.sections.iter().flatten().count() * (19 + 16 * 16 * 16 + 16 * base),
  );
  let mut chunk_buf = Buffer::new(&mut chunk_data);
  for s in chunk.sections.iter().flatten() {
    chunk_buf.write_u8(s.data().bpe() as u8);
    chunk_buf.write_varint(s.palette().len() as i32);
    for g in s.palette() {
      chunk_buf.write_varint(conv.block_to_old(*g as u32, ver.block()) as i32);
    }
    let longs = s.data().old_long_array();
    chunk_buf.write_varint(longs.len() as i32);
    chunk_buf.reserve(longs.len() * 8); // 8 bytes per long
    chunk_buf.write_buf(unsafe {
      std::slice::from_raw_parts(longs.as_ptr() as *const u8, longs.len() * 8)
    });
    // Light data
    chunk_buf.reserve(16 * base);
    for _ in 0..16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      chunk_buf.write_u8(0xff);
    }
    if skylight {
      chunk_buf.reserve(16 * base);
      for _ in 0..16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        chunk_buf.write_u8(0xff);
      }
    }
  }

  if biomes {
    chunk_buf.reserve(256);
    for _ in 0..256 {
      chunk_buf.write_u8(127); // Void biome
    }
  }

  let mut data = Vec::with_capacity(4 + chunk_buf.len());
  let mut buf = Buffer::new(&mut data);
  buf.write_varint(chunk_buf.len() as i32);
  buf.write_buf(&chunk_data);

  // No block entities
  if ver >= ProtocolVersion::V1_9_4 {
    buf.write_varint(0);
  }

  packet::ChunkDataV9 {
    chunk_x:            chunk.pos.x(),
    chunk_z:            chunk.pos.z(),
    load_chunk:         chunk.full,
    available_sections: chunk.old_bit_map().into(),
    unknown:            data,
    v_2:                0,
  }
  .into()
}

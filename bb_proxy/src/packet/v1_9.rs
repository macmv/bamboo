use super::{ChunkWithPos, TypeConverter};
use crate::gnet::cb::{packet, Packet};
use bb_common::{util::Buffer, version::ProtocolVersion};

// Applies to 1.9 - 1.12, but 1.10 doesn't work, so idk
pub fn chunk(chunk: ChunkWithPos, ver: ProtocolVersion, conv: &TypeConverter) -> Packet {
  let biomes = chunk.full;
  let skylight = true; // Assume overworld

  let base = 16 * 16 / 2;
  let total_sections = chunk
    .sections
    .iter()
    .zip(chunk.block_light.sections().iter())
    .filter(|(s, l)| s.is_some() || l.is_some())
    .count();
  let mut chunk_data = Vec::with_capacity(1024 + total_sections * (19 + 16 * 16 * 16 + 16 * base));
  let mut chunk_buf = Buffer::new(&mut chunk_data);
  for (block_section, light_section) in
    chunk.sections.iter().zip(chunk.block_light.sections().iter())
  {
    if block_section.is_none() && light_section.is_none() {
      continue;
    }

    if let Some(s) = block_section {
      chunk_buf.write_u8(s.data().bpe());
      chunk_buf.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_buf.write_varint(conv.block_to_old(*g, ver.block()) as i32);
      }
      let longs = s.data().old_long_array();
      chunk_buf.write_varint(longs.len() as i32);
      chunk_buf.reserve(longs.len() * 8); // 8 bytes per long
      longs.iter().for_each(|v| chunk_buf.write_buf(&v.to_be_bytes()));
    } else {
      // Write an empty section
      chunk_buf.write_u8(4); // 4 bits per entry
      chunk_buf.write_varint(1); // Palette has 1 entry
      chunk_buf.write_varint(0); // The entry is `0`

      // each entry is 4 bits
      chunk_buf.write_varint(256); // there are 256 longs (16 * 16 * 16 / 2 / 8)
      chunk_buf.reserve(16 * 16 * 16 / 2);
      for _ in 0..16 * 16 * 16 / 2 {
        chunk_buf.write_u8(0x00);
      }
    }
    // Block light data
    if let Some(l) = light_section {
      chunk_buf.write_buf(l.data());
    } else {
      // Each lighting value is 1/2 byte
      chunk_buf.reserve(16 * 16 * 16 / 2);
      for _ in 0..16 * 16 * 16 / 2 {
        chunk_buf.write_u8(0x00);
      }
    }

    if skylight {
      chunk_buf.reserve(16 * base);
      for _ in 0..16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        chunk_buf.write_u8(0x00);
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

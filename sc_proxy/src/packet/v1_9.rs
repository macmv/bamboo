use super::TypeConverter;
use crate::gnet::cb::Packet;
use sc_common::{chunk::paletted::Section, math::ChunkPos, util::Buffer, version::ProtocolVersion};

// Applies to 1.9 - 1.12, but 1.10 doesn't work, so idk
pub fn chunk(
  pos: ChunkPos,
  full: bool,
  bit_map: u16,
  sections: &[Section],
  ver: ProtocolVersion,
  conv: &TypeConverter,
) -> Packet {
  let biomes = full;
  let skylight = true; // Assume overworld

  let mut chunk_data = vec![];
  let mut chunk_buf = Buffer::new(&mut chunk_data);
  // Iterates through chunks in order, from ground up. Flatten removes None
  // sections.
  for s in sections {
    chunk_buf.write_u8(s.data().bpe() as u8);
    chunk_buf.write_varint(s.palette().len() as i32);
    for g in s.palette() {
      chunk_buf.write_varint(conv.block_to_old(*g as u32, ver.block()) as i32);
    }
    let longs = s.data().long_array();
    chunk_buf.write_varint(longs.len() as i32);
    chunk_buf.reserve(longs.len() * 8); // 8 bytes per long
    longs.iter().for_each(|v| chunk_buf.write_buf(&v.to_be_bytes()));
    // Light data
    for _ in 0..16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      chunk_buf.write_u8(0xff);
    }
    if skylight {
      for _ in 0..16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        chunk_buf.write_u8(0xff);
      }
    }
  }

  if biomes {
    for _ in 0..256 {
      chunk_buf.write_u8(127); // Void biome
    }
  }

  let mut data = Vec::with_capacity(chunk_buf.len());
  let mut buf = Buffer::new(&mut data);
  buf.write_varint(chunk_buf.len() as i32);
  buf.write_buf(&chunk_data);

  // No block entities
  if ver >= ProtocolVersion::V1_9_4 {
    buf.write_varint(0);
  }

  Packet::ChunkDataV9 {
    chunk_x:            pos.x(),
    chunk_z:            pos.z(),
    load_chunk:         full,
    available_sections: bit_map.into(),
    buffer:             vec![],
    field_189557_e:     None,
    unknown:            data,
  }
}

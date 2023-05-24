use super::{ChunkWithPos, TypeConverter};
use crate::gnet::cb::{packet, Packet};
use bb_common::{
  chunk::Section as _,
  math::{ChunkPos, SectionRelPos},
  util::Buffer,
  version::{BlockVersion, ProtocolVersion},
};

pub fn chunk(chunk: ChunkWithPos, conv: &TypeConverter) -> Packet {
  let biomes = chunk.full;
  let skylight = true; // Assume overworld

  // Don't send unload chunks when we really want an empty chunk.
  let actual_sections = chunk.sections.iter().flatten().count();
  let total_sections = if actual_sections == 0 { 1 } else { actual_sections };

  let data_len = total_sections * 16 * 16 * 16 * 2 // Chunk data
    + (total_sections * 16 * 16 * 16 / 2) // Block light
    + if skylight { total_sections * 16 * 16 * 16 / 2 } else { 0 } // Sky light
    + if biomes { 256 } else { 0 }; // Biomes

  // The most it will be is data_len + max varint len
  let mut chunk_data = vec![0; data_len + 2 + 5];
  let mut chunk_buf = Buffer::new(&mut chunk_data);

  chunk_buf.write_u16(if actual_sections == 0 { 1 } else { chunk.old_bit_map() });
  chunk_buf.write_varint(data_len.try_into().unwrap());
  let prefix_len = chunk_buf.index();

  if actual_sections == 0 {
    // We just want 4096 0_u16s for this section.
    chunk_buf.skip(16 * 16 * 16 * 2);
  } else {
    for s in chunk.sections.iter().flatten() {
      for y in 0..16 {
        for z in 0..16 {
          for x in 0..16 {
            let b = s.get_block(SectionRelPos::new(x, y, z));
            // Theres a lot of air. Profiling says this helps a lot (~20% improvement for a
            // superflat world).
            if b == 0 {
              chunk_buf.skip(2);
              continue;
            }
            let old_id = conv.block_to_old(b, BlockVersion::V1_8);
            chunk_buf.write_buf(&(old_id as u16).to_le_bytes());
          }
        }
      }
    }
  }
  // Light data
  for _ in 0..total_sections * 16 * 16 * 16 / 2 {
    // Each lighting value is 1/2 byte
    chunk_buf.write_u8(0xff);
  }
  if skylight {
    for _ in 0..total_sections * 16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      chunk_buf.write_u8(0xff);
    }
  }
  if biomes {
    for _ in 0..256 {
      chunk_buf.write_u8(127); // Void biome
    }
  }
  // This is going to pop at most 4 elements.
  let len = chunk_buf.index();
  chunk_data.truncate(len);
  assert_eq!(chunk_data.len() - prefix_len, data_len, "unexpected chunk data len");

  Packet::ChunkData(packet::ChunkData::V8(packet::ChunkDataV8 {
    chunk_x:        chunk.pos.x(),
    chunk_z:        chunk.pos.z(),
    field_149279_g: chunk.full,
    unknown:        chunk_data,
  }))
}

pub fn multi_block_change(
  pos: ChunkPos,
  y: i32,
  changes: Vec<u64>,
  ver: ProtocolVersion,
  conv: &TypeConverter,
) -> Packet {
  let mut data = vec![];
  let mut buf = Buffer::new(&mut data);
  buf.write_i32(pos.x());
  buf.write_i32(pos.z());
  buf.write_varint(changes.len() as i32);
  for change in changes {
    let id = (change >> 12) as u32;
    let s_x = ((change >> 8) & 0xf) as u8;
    let s_y = ((change >> 4) & 0xf) as u8;
    let s_z = ((change >> 0) & 0xf) as u8;
    let old_id = conv.block_to_old(id, ver.block());
    let y = y * 16 + s_y as i32;

    buf.write_u8(s_x << 4 | s_z);
    buf.write_u8(y as u8);
    buf.write_varint(old_id as i32);
  }
  packet::MultiBlockChangeV8 { unknown: data }.into()
}

use super::TypeConverter;
use crate::gnet::cb::Packet;
use sc_common::{
  chunk::{paletted::Section, Section as _},
  math::{ChunkPos, Pos},
  util::Buffer,
  version::{BlockVersion, ProtocolVersion},
};

pub fn chunk(
  pos: ChunkPos,
  full: bool,
  bit_map: u16,
  sections: &[Section],
  conv: &TypeConverter,
) -> Packet {
  let biomes = full;
  let skylight = true; // Assume overworld

  let total_sections = sections.len();

  let data_len = total_sections * 16 * 16 * 16 * 2 // Chunk data
    + (total_sections * 16 * 16 * 16 / 2) // Block light
    + if skylight { total_sections * 16 * 16 * 16 / 2 } else { 0 } // Sky light
    + if biomes { 256 } else { 0 }; // Biomes

  // The most it will be is data_len + max varint len
  let mut chunk_data = Buffer::new(Vec::with_capacity(data_len + 5));

  chunk_data.write_u16(bit_map);
  chunk_data.write_varint(data_len.try_into().unwrap());
  let prefix_len = chunk_data.len();

  for s in sections {
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          let b = s.get_block(Pos::new(x, y, z)).unwrap();
          let old_id = conv.block_to_old(b, BlockVersion::V1_8);
          chunk_data.write_buf(&(old_id as u16).to_le_bytes());
        }
      }
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

  Packet::ChunkDataV8 {
    chunk_x:        pos.x(),
    chunk_z:        pos.z(),
    field_149279_g: full,
    extracted_data: None,
    unknown:        chunk_data.into_inner(),
  }
}

pub fn multi_block_change(
  pos: ChunkPos,
  y: i32,
  changes: Vec<u64>,
  ver: ProtocolVersion,
  conv: &TypeConverter,
) -> Packet {
  let mut buf = Buffer::new(vec![]);
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

    buf.write_u8((s_x as u8) << 4 | s_z as u8);
    buf.write_u8(y as u8);
    buf.write_varint(old_id as i32);
  }
  Packet::MultiBlockChangeV8 {
    chunk_pos_coord: None,
    changed_blocks:  vec![],
    unknown:         buf.into_inner(),
  }
}

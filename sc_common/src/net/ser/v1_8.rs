use crate::{
  chunk::{paletted::Section, Section as _},
  math::Pos,
  net::VersionConverter,
  util::Buffer,
};
use sc_generated::{net::cb::Packet, version::BlockVersion};

pub fn chunk(
  x: i32,
  z: i32,
  bit_map: u16,
  sections: &[Section],
  conv: &impl VersionConverter,
) -> Packet {
  let skylight = true;
  let biomes = true;

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
    // chunk_data.write_buf(&s.data().iter().map(|v|
    // v.to_le_bytes()).flatten().collect::<Vec<u8>>());
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
    chunk_x:        x,
    chunk_z:        z,
    field_149279_g: true,
    extracted_data: None,
    unknown:        chunk_data.into_inner(),
  }
}

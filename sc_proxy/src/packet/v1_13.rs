use crate::{
  chunk::paletted::Section,
  math::ChunkPos,
  net::{cb, VersionConverter},
  util::Buffer,
  version::BlockVersion,
};
use sc_generated::net::cb::Packet;

// CHANGES:
// No length is written for >8 bpb
// Biome is i32, not u8
pub fn chunk(
  pos: ChunkPos,
  bit_map: u16,
  sections: &[Section],
  conv: &impl VersionConverter,
) -> Packet {
  let biomes = true; // Always true with new chunk set
  let skylight = true; // Assume overworld

  let mut chunk_data = Buffer::new(vec![]);
  // Iterates through chunks in order, from ground up. Flatten removes None
  // sections.
  for s in sections {
    chunk_data.write_u8(s.data().bpe() as u8);
    if s.data().bpe() <= 8 {
      chunk_data.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_data.write_varint(conv.block_to_old(*g as u32, BlockVersion::V1_13) as i32);
      }
    }
    let longs = s.data().long_array();
    chunk_data.write_varint(longs.len() as i32);
    longs.iter().for_each(|v| chunk_data.write_buf(&v.to_be_bytes()));
    // Light data
    for _ in 0..16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      chunk_data.write_u8(0xff);
    }
    if skylight {
      for _ in 0..16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        chunk_data.write_u8(0xff);
      }
    }
  }

  if biomes {
    for _ in 0..256 {
      chunk_data.write_i32(127); // Void biome
    }
  }

  let mut data = Buffer::new(Vec::with_capacity(chunk_data.len()));
  data.write_varint(chunk_data.len() as i32);
  data.write_buf(&chunk_data.into_inner());
  Packet::ChunkDataV14 {
    x:                                     pos.x(),
    z:                                     pos.z(),
    ground_up:                             true,
    bit_map_v1_8:                          None,
    bit_map_v1_9:                          Some(bitmask),
    chunk_data:                            data.into_inner(),
    block_entities_v1_9_4:                 Some(vec![0]), // 0 len
    heightmaps_v1_14:                      None,
    biomes_v1_15:                          None,
    biomes_v1_16_2:                        None,
    ignore_old_data_v1_16_removed_v1_16_2: None,
  }
}

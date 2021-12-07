use crate::{chunk::paletted::Section, math::ChunkPos, net::VersionConverter, util::Buffer};
use sc_generated::{net::cb::Packet, version::BlockVersion};

// Applies to 1.9 - 1.12, but 1.10 doesn't work, so idk
pub fn chunk(
  pos: ChunkPos,
  bit_map: u16,
  sections: &[Section],
  ver: BlockVersion,
  conv: &impl VersionConverter,
) -> Packet {
  let biomes = true; // Always true with new chunk set
  let skylight = true; // Assume overworld

  let mut chunk_data = Buffer::new(vec![]);
  // Iterates through chunks in order, from ground up. Flatten removes None
  // sections.
  for s in sections {
    chunk_data.write_u8(s.data().bpe() as u8);
    chunk_data.write_varint(s.palette().len() as i32);
    for g in s.palette() {
      chunk_data.write_varint(conv.block_to_old(*g as u32, ver) as i32);
    }
    let longs = s.data().long_array();
    chunk_data.write_varint(longs.len() as i32);
    chunk_data.write_buf(&longs.iter().map(|v| v.to_be_bytes()).flatten().collect::<Vec<u8>>());
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
      chunk_data.write_u8(127); // Void biome
    }
  }

  let mut data = Buffer::new(Vec::with_capacity(chunk_data.len()));
  data.write_varint(chunk_data.len() as i32);
  data.write_buf(&chunk_data.into_inner());
  Packet::ChunkDataV9 {
    chunk_x:            pos.x(),
    chunk_z:            pos.z(),
    load_chunk:         true,
    available_sections: bit_map.into(),
    buffer:             vec![],
    field_189557_e:     None,
    unknown:            data.into_inner(),
  }
}

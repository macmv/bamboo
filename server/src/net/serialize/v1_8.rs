use crate::world::chunk::MultiChunk;
use common::{math::ChunkPos, net::cb, util::Buffer};

pub fn serialize_chunk(pos: ChunkPos, c: &MultiChunk) -> cb::Packet {
  let mut chunk_data = Buffer::new(vec![]);
  let mut bit_map = 0;
  let c = c.get_fixed();

  let skylight = true;
  let biomes = true;

  let mut total_sections = 0;
  // Flatten removes all the None chunks
  for (y, s) in c.sections().into_iter().enumerate() {
    if let Some(s) = s {
      let s = s.unwrap_fixed();
      bit_map |= 1 << y;
      total_sections += 1;
      // These are little endian. I don't know why. It probably has something to do
      // with the way I serialize things, but I couldn't really be bothered to figure
      // it out (because it works).
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
  cb::Packet::MapChunk {
    x:                     pos.x(),
    z:                     pos.z(),
    ground_up:             true,
    chunk_data:            chunk_data.into_inner(),
    bit_map_v1_8:          Some(bit_map),
    bit_map_v1_9:          None,
    block_entities_v1_9_4: None,
    heightmaps_v1_14:      None,
    biomes_v1_15:          None,
    biomes_v1_16_2:        None,
    ignore_old_data_v1_16: None,
  }
}

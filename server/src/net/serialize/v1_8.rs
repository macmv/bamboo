use crate::world::chunk::MultiChunk;
use common::{math::ChunkPos, net::cb};

pub fn serialize_chunk(pos: ChunkPos, c: &MultiChunk) -> cb::Packet {
  let mut chunk_data = vec![];
  let mut bit_map = 0;
  cb::Packet::MapChunk {
    x: pos.x(),
    z: pos.z(),
    ground_up: true,
    chunk_data,
    bit_map_v1_8: Some(bit_map),
    bit_map_v1_9: None,
    block_entities_v1_9_4: None,
    heightmaps_v1_14: None,
    biomes_v1_15: None,
    biomes_v1_16_2: None,
    ignore_old_data_v1_16: None,
  }
}

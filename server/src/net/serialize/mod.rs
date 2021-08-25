use crate::world::chunk::MultiChunk;
use common::{math::ChunkPos, net::cb, version::BlockVersion};

mod v1_8;
mod v1_9;

pub fn serialize_chunk(pos: ChunkPos, c: &MultiChunk, ver: BlockVersion) -> cb::Packet {
  match ver {
    BlockVersion::V1_8 => v1_8::serialize_chunk(pos, c),
    BlockVersion::V1_9 => v1_9::serialize_chunk(pos, c),
    ver => unimplemented!("cannot serialize chunks for version {:?}", ver),
  }
}

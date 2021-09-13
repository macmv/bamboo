use crate::{block, world::chunk::MultiChunk};
use sc_common::{math::ChunkPos, net::cb, version::BlockVersion};

mod v1_13;
mod v1_14;
mod v1_8;
mod v1_9;

pub fn serialize_chunk(pos: ChunkPos, c: &MultiChunk, ver: BlockVersion) -> cb::Packet {
  match ver {
    BlockVersion::V1_8 => v1_8::serialize_chunk(pos, c),
    BlockVersion::V1_9 | BlockVersion::V1_12 => v1_9::serialize_chunk(pos, c, ver),
    BlockVersion::V1_13 => v1_13::serialize_chunk(pos, c),
    BlockVersion::V1_14 => v1_14::serialize_chunk(pos, c),
    ver => unimplemented!("cannot serialize chunks for version {:?}", ver),
  }
}
pub fn serialize_multi_block_change<I>(pos: ChunkPos, ver: BlockVersion, changes: I) -> cb::Packet
where
  I: Iterator<Item = ((u8, u8), block::Type)>,
{
  match ver {
    // BlockVersion::V1_8 => v1_8::serialize_multi_block_change(pos, changes),
    // BlockVersion::V1_9 | BlockVersion::V1_12 => v1_9::serialize_multi_block_change(pos, c, ver),
    // BlockVersion::V1_13 => v1_13::serialize_multi_block_change(pos, c),
    // BlockVersion::V1_14 => v1_14::serialize_multi_block_change(pos, c),
    ver => unimplemented!("cannot serialize multi block change for version {:?}", ver),
  }
}

pub fn serialize_partial_chunk(
  pos: ChunkPos,
  c: &MultiChunk,
  ver: BlockVersion,
  min: u32,
  max: u32,
) -> cb::Packet {
  match ver {
    BlockVersion::V1_8 => v1_8::serialize_partial_chunk(pos, c, min, max),
    ver => unimplemented!("cannot serialize chunks for version {:?}", ver),
  }
}

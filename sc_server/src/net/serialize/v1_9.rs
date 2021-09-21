use crate::world::chunk::MultiChunk;
use sc_common::{math::ChunkPos, net::cb, util::Buffer, version::BlockVersion};

// Applies to 1.9 - 1.12, but 1.10 doesn't work, so idk
pub fn serialize_chunk(pos: ChunkPos, c: &MultiChunk, ver: BlockVersion) -> cb::Packet {
  let types = c.type_converter();
  let c = c.get_paletted();

  let biomes = true; // Always true with new chunk set
  let skylight = true; // Assume overworld

  let mut bitmask = 0;
  for (y, section) in c.sections().enumerate() {
    if section.is_some() {
      bitmask |= 1 << y;
    }
  }

  let mut chunk_data = Buffer::new(vec![]);
  // Iterates through chunks in order, from ground up. Flatten removes None
  // sections.
  for s in c.sections().flatten() {
    let s = s.unwrap_paletted();
    chunk_data.write_u8(s.data().bpe() as u8);
    chunk_data.write_varint(s.palette().len() as i32);
    for g in s.palette() {
      chunk_data.write_varint(types.to_old(*g as u32, ver) as i32);
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
  cb::Packet::MapChunk {
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

pub fn serialize_partial_chunk(
  pos: ChunkPos,
  c: &MultiChunk,
  _min: u32,
  _max: u32,
  ver: BlockVersion,
) -> cb::Packet {
  let types = c.type_converter();
  let c = c.get_paletted();

  let biomes = false; // Always false, because we are just sending some new blocks
  let skylight = true; // Assume overworld

  let mut bitmask = 0;
  let mut chunk_data = Buffer::new(vec![]);
  // Iterates through chunks in order, from ground up. Flatten removes None
  // sections.
  for (y, s) in c.sections().into_iter().enumerate() {
    if let Some(s) = s {
      let s = s.unwrap_paletted();
      bitmask |= 1 << y;
      chunk_data.write_u8(s.data().bpe() as u8);
      chunk_data.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_data.write_varint(types.to_old(*g as u32, ver) as i32);
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
  }

  if biomes {
    for _ in 0..256 {
      chunk_data.write_u8(127); // Void biome
    }
  }

  let mut data = Buffer::new(Vec::with_capacity(chunk_data.len()));
  data.write_varint(chunk_data.len() as i32);
  data.write_buf(&chunk_data.into_inner());
  cb::Packet::MapChunk {
    x:                                     pos.x(),
    z:                                     pos.z(),
    ground_up:                             false, // Because partial chunks
    chunk_data:                            data.into_inner(),
    bit_map_v1_8:                          None,
    bit_map_v1_9:                          Some(bitmask),
    block_entities_v1_9_4:                 Some(vec![0]),
    heightmaps_v1_14:                      None,
    biomes_v1_15:                          None,
    biomes_v1_16_2:                        None,
    ignore_old_data_v1_16_removed_v1_16_2: None,
  }
}

use std::{collections::HashMap, convert::TryInto};

use super::{utils, PacketSpec};

use common::{
  net::{cb, Other},
  proto,
  util::Buffer,
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::PlayerInfo, utils::generate_player_info);
  spec.add(cb::ID::MapChunk, |gen, v, p| {
    let mut chunk = proto::Chunk { x: p.read_i32(), z: p.read_i32(), ..Default::default() };
    let _new_chunk = p.read_bool();

    let biomes = true; // Always true with new chunk set
    let skylight = true; // Assume overworld

    let bitmask = p.read_u16();
    let data_len = p.read_varint();
    let mut data = Buffer::new(p.read_buf(data_len));

    // Makes an ordered list of chunk sections
    let mut sections = vec![];
    for y in 0..16 {
      if bitmask & 1 << y != 0 {
        // 4096 blocks, 2 bytes per block;
        let u8_buf = data.read_buf(16 * 16 * 16 * 2);
        let mut data = vec![0; u8_buf.len() / 8];
        for (i, b) in data.iter_mut().enumerate() {
          *b = u64::from_le_bytes(u8_buf[i * 8..i * 8 + 8].try_into().unwrap());
        }
        sections.push((y, proto::chunk::Section { data, ..Default::default() }));
      }
    }
    chunk.sections = sections.into_iter().collect();
    for y in 0..16 {
      if bitmask & 1 << y != 0 {
        // Light data (1/2 byte per block)
        let block_light = data.read_buf(16 * 16 * 16 / 2);
      }
    }
    if skylight {
      for y in 0..16 {
        if bitmask & 1 << y != 0 {
          let sky_light = data.read_buf(16 * 16 * 16 / 2);
        }
      }
    }
    if biomes {
      // One biome per column
      let biomes = data.read_buf(256);
    }

    assert!(data.read_all().is_empty(), "should have read all the chunk data above");

    // Not needed. Leaving commented out for reference
    //
    // expected := num_sections * 16*16*16 * 2 // Block data
    // expected += num_sections * 16*16*16 / 2 // Block light data
    // if skylight {
    //   expected += num_sections * 16*16*16 / 2 // Sky light data
    // }
    // if biomes {
    //   expected += 256 // Biome data
    // }
    // if buf.Length() != int32(expected) {
    //   fmt.Println("ERROR: Incorrectly serialized chunk! Expected length:",
    // expected, "actual length:", buf.Length()) }

    let mut out = cb::Packet::new(cb::ID::MapChunk);
    out.set_other(Other::Chunk(chunk));
    Ok(vec![out])
  });
  spec
}

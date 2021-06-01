use std::{
  collections::HashMap,
  io::{Error, ErrorKind},
};

use super::{utils, PacketSpec};
use crate::packet::Packet;

use common::{
  net::{cb, Other},
  util::{
    nbt::{Tag, NBT},
    Buffer,
  },
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::PlayerInfo, utils::generate_player_info);
  spec.add(cb::ID::MapChunk, |mut out: Packet, p: &cb::Packet| {
    // TODO: Error handling should be done within the packet.
    let chunk = match p.read_other().unwrap() {
      Other::Chunk(c) => c,
      o => return Err(Error::new(ErrorKind::InvalidData, format!("expected chunk, got {:?}", o))),
    };
    out.write_i32(chunk.x);
    out.write_i32(chunk.z);
    out.write_bool(true); // Always a new chunk

    let biomes = true; // Always true with new chunk set
    let skylight = true; // Assume overworld

    let mut bitmask = 0;
    for y in chunk.sections.keys() {
      bitmask |= 1 << y;
    }
    out.write_varint(bitmask);

    out.write_buf(
      &NBT::new(
        "",
        Tag::Compound(vec![NBT::new("MOTION_BLOCKING", Tag::LongArray(chunk.heightmap))]),
      )
      .serialize(),
    );

    let mut buf = Buffer::new(vec![]);
    // Makes an ordered list of chunk sections
    let mut sections = vec![None; 16];
    for (y, s) in &chunk.sections {
      sections[*y as usize] = Some(s);
    }
    // Iterates through chunks in order, from ground up. flatten() skips all None
    // sections.
    for s in sections.into_iter().flatten() {
      buf.write_u16(s.non_air_blocks as u16);
      buf.write_u8(s.bits_per_block as u8);
      if s.bits_per_block <= 8 {
        // The length of the palette
        buf.write_varint(s.palette.len() as i32);
        for g in &s.palette {
          buf.write_varint(*g as i32);
        }
      }
      // Number of longs in the data array
      buf.write_varint(s.data.len() as i32);
      buf.write_buf(&s.data.iter().map(|v| v.to_be_bytes()).flatten().collect::<Vec<u8>>());
      // Light data is now sent in another packet
    }

    if biomes {
      for _ in 0..256 {
        buf.write_i32(127); // Void biome
      }
    }

    // if chunk.x == 3 && chunk.z == 5 {
    //   println!("{:x?}", buf);
    // }

    out.write_varint(buf.len() as i32);
    out.write_buf(&buf);
    // No tile entities
    out.write_varint(0);

    Ok(Some(out))
  });
  spec
}

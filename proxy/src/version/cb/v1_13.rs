use std::{
  collections::HashMap,
  io::{Error, ErrorKind},
};

use super::{utils, PacketSpec};
use crate::packet::Packet;

use common::{
  net::{cb, Other},
  util::Buffer,
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::PlayerInfo, utils::generate_player_info);
  spec.add(cb::ID::MapChunk, |gen, v, p| {
    let mut out = Packet::new(gen.convert_id(v, cb::ID::MapChunk), v);
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

    let mut buf = Buffer::new(vec![]);
    // Makes an ordered list of chunk sections
    let mut sections = vec![None; 16];
    for (y, s) in &chunk.sections {
      sections[*y as usize] = Some(s);
    }
    // Iterates through chunks in order, from ground up. flatten() skips all None
    // sections.
    for s in sections.into_iter().flatten() {
      // The bits per block
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
      // Light data
      for _ in 0..16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        buf.write_u8(0xff);
      }
      if skylight {
        for _ in 0..16 * 16 * 16 / 2 {
          // Each lighting value is 1/2 byte
          buf.write_u8(0xff);
        }
      }
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

    Ok(vec![out])
  });
  spec
}

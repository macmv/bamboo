use std::{
  collections::HashMap,
  io::{Error, ErrorKind},
};

use super::{utils, PacketSpec};

use common::{
  net::{cb, tcp, Other},
  util::{
    nbt::{Tag, NBT},
    Buffer,
  },
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::PlayerInfo, utils::generate_player_info);
  spec.add(cb::ID::DeclareCommands, utils::generate_declare_commands);
  spec.add(cb::ID::MapChunk, |gen, v, p| {
    let mut out = tcp::Packet::new(gen.convert_id(v, cb::ID::MapChunk), v);
    let mut light = tcp::Packet::new(gen.convert_id(v, cb::ID::UpdateLight), v);
    // TODO: Error handling should be done within the packet.
    let chunk = match p.read_other().unwrap() {
      Other::Chunk(c) => c,
      o => return Err(Error::new(ErrorKind::InvalidData, format!("expected chunk, got {:?}", o))),
    };
    out.write_i32(chunk.x);
    out.write_i32(chunk.z);
    light.write_varint(chunk.x);
    light.write_varint(chunk.z);
    out.write_bool(true); // Always a new chunk

    let biomes = true; // Always true with new chunk set
    let _skylight = true; // Assume overworld

    let mut bitmask = 0;
    for y in chunk.sections.keys() {
      bitmask |= 1 << y;
    }
    out.write_varint(bitmask);
    // TODO: Send light data
    light.write_bool(false); // Trust edges
    light.write_varint(bitmask << 1); // Sky light mask (0 bit is blocks -16 to -1, so we << 1)
    light.write_varint(bitmask << 1); // Block light mask
    light.write_varint(0); // Empty sky light mask
    light.write_varint(0); // Empty block light mask

    out.write_buf(
      &NBT::new("", Tag::compound(&[("MOTION_BLOCKING", Tag::LongArray(chunk.heightmap))]))
        .serialize(),
    );

    if biomes {
      out.write_varint(1024);
      for _ in 0..1024 {
        out.write_varint(0); // Custom biome
      }
    }

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
        buf.write_varint(s.palette.len() as i32);
        for g in &s.palette {
          buf.write_varint(*g as i32);
        }
      }
      // Number of longs in the data array
      buf.write_varint(s.data.len() as i32);
      buf.write_buf(&s.data.iter().map(|v| v.to_be_bytes()).flatten().collect::<Vec<u8>>());

      // Sky light
      light.write_varint(2048);
      for _ in 0..16 * 16 * 16 / 2 {
        light.write_u8(0xff);
      }
      // Block light
      light.write_varint(2048);
      for _ in 0..16 * 16 * 16 / 2 {
        light.write_u8(0xff);
      }
    }

    // if chunk.x == 3 && chunk.z == 5 {
    //   println!("{:x?}", buf);
    // }

    out.write_varint(buf.len() as i32);
    out.write_buf(&buf);
    // No tile entities
    out.write_varint(0);

    Ok(vec![out, light])
  });
  spec
}

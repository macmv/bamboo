use std::{
  collections::HashMap,
  io::{Error, ErrorKind},
};

use super::PacketSpec;
use crate::packet::Packet;

use common::{
  net::{cb, Other},
  util::Buffer,
  version::ProtocolVersion,
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::KeepAlive, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x00, v);
    out.write_varint(p.get_int("keep_alive_id")?); // Keep alive id
    Ok(out)
  });
  spec.add(cb::ID::JoinGame, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x01, v);
    out.write_i32(p.get_int("eid")?); // EID
    out.write_u8(p.get_byte("gamemode")?); // Gamemode
    out.write_u8(0); // Dimension. TODO: Cross dimension stuff with the new codec system
    out.write_u8(1); // Difficulty
    out.write_u8(0); // Max players (ignored)
    out.write_str("default"); // Level type
    out.write_bool(p.get_bool("reduced_debug_info")?);
    Ok(out)
  });
  spec.add(cb::ID::ChatMessage, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x02, v);
    out.write_str(p.get_str("message")?); // Message, json encoded
    out.write_u8(p.get_byte("position")?); // Position 0: chat box, 1: system message, 2: game info (above hotbar)
    Ok(out)
  });
  spec.add(cb::ID::TimeUpdate, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x03, v);
    out.write_u64(p.get_long("age")?); // World age
    out.write_u64(p.get_long("time")?); // Time of day
    Ok(out)
  });
  spec.add(cb::ID::SpawnPosition, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x05, v);
    out.write_pos(p.get_pos("location")?); // The location that your compass points to
    Ok(out)
  });
  spec.add(cb::ID::UpdateHealth, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x06, v);
    out.write_f32(p.get_float("health")?); // Health
    out.write_varint(p.get_int("food")?); // Food
    out.write_f32(p.get_float("saturation")?); // Saturation
    Ok(out)
  });
  spec.add(cb::ID::Respawn, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x07, v);
    out.write_i32(p.get_int("dimension")?); // Dimension
    out.write_u8(1); // Difficulty
    out.write_u8(p.get_byte("gamemode")?); // Gamemode
    out.write_str("default"); // Level type
    Ok(out)
  });
  spec.add(cb::ID::PlayerPositionAndLook, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x08, v);
    out.write_f64(p.get_double("x")?); // X
    out.write_f64(p.get_double("y")?); // Y
    out.write_f64(p.get_double("z")?); // Z
    out.write_f32(p.get_float("yaw")?); // Yaw
    out.write_f32(p.get_float("pitch")?); // Pitch
    out.write_u8(p.get_byte("flags")?); // Flags
    Ok(out)
  });
  spec.add(cb::ID::HeldItemChange, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x09, v);
    out.write_u8(p.get_byte("slot")?); // Slot
    Ok(out)
  });
  spec.add(cb::ID::ChunkData, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x21, v);
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
    out.write_u16(bitmask);

    let mut buf = Buffer::new(vec![]);
    // Makes an ordered list of chunk sections
    let mut sections = vec![None; 16];
    for (y, s) in &chunk.sections {
      sections[*y as usize] = Some(s);
    }
    // Iterates through chunks in order, from ground up. flatten() skips all None
    // sections.
    let mut total_sections = 0;
    for s in sections.into_iter().flatten() {
      total_sections += 1;
      buf.write_buf(&s.data);
    }
    // Light data
    for _ in 0..total_sections * 16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      buf.write_u8(0xff);
    }
    if skylight {
      for _ in 0..total_sections * 16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        buf.write_u8(0xff);
      }
    }
    if biomes {
      for _ in 0..256 {
        buf.write_u8(127); // Void biome
      }
    }

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
    out.write_varint(buf.len() as i32);
    out.write_buf(&buf);

    Ok(out)
  });
  spec
}

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
    out.write_varint(p.pb().ints[0]); // Keep alive id
    Ok(out)
  });
  spec.add(cb::ID::JoinGame, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x01, v);
    out.write_i32(p.pb().ints[0]); // EID
    out.write_u8(p.pb().bytes[0]); // Gamemode
    out.write_u8(0); // Dimension. TODO: Cross dimension stuff with the new codec system
    out.write_u8(1); // Difficulty
    out.write_u8(0); // Max players (ignored)
    out.write_str("default"); // Level type
    out.write_bool(p.pb().bools[0]); // Reduced debug info
    Ok(out)
  });
  spec.add(cb::ID::ChatMessage, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x02, v);
    out.write_str(&p.pb().strs[0]); // Message, json encoded
    out.write_u8(p.pb().bytes[0]); // Position 0: chat box, 1: system message, 2: game info (above hotbar)
    Ok(out)
  });
  spec.add(cb::ID::TimeUpdate, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x03, v);
    out.write_u64(p.pb().longs[0]); // World age
    out.write_u64(p.pb().longs[1]); // Time of day
    Ok(out)
  });
  spec.add(cb::ID::EntityEquipment, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x04, v);
    out.write_varint(p.pb().ints[0]); // EID
    out.write_i16(p.pb().shorts[0] as i16); // EID
    out.write_u64(p.pb().longs[1]); // Time of day
    Ok(out)
  });
  spec.add(cb::ID::SpawnPosition, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x05, v);
    out.write_pos(p.pb().longs[0]); // The location that your compass points to
    Ok(out)
  });
  spec.add(cb::ID::UpdateHealth, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x06, v);
    out.write_f32(p.pb().floats[0]); // Health
    out.write_varint(p.pb().ints[0]); // Food
    out.write_f32(p.pb().floats[0]); // Saturation
    Ok(out)
  });
  spec.add(cb::ID::Respawn, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x07, v);
    out.write_i32(p.pb().ints[0]); // Dimension
    out.write_u8(1); // Difficulty
    out.write_u8(p.pb().bytes[0]); // Gamemode
    out.write_str("default"); // Level type
    Ok(out)
  });
  spec.add(cb::ID::PlayerPositionAndLook, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x08, v);
    out.write_f64(p.pb().doubles[0]); // X
    out.write_f64(p.pb().doubles[1]); // Y
    out.write_f64(p.pb().doubles[2]); // Z
    out.write_f32(p.pb().floats[0]); // Yaw
    out.write_f32(p.pb().floats[1]); // Pitch
    out.write_u8(p.pb().bytes[0]); // Flags
    Ok(out)
  });
  spec.add(cb::ID::HeldItemChange, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x09, v);
    out.write_u8(p.pb().bytes[0]); // Slot
    Ok(out)
  });
  // Use bed doesn't exist in newer versions
  spec.add(cb::ID::EntityAnimation, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x0b, v);
    out.write_varint(p.pb().ints[0]); // Player's ID (eid? not sure)
    out.write_u8(p.pb().bytes[0]); // Animation
    Ok(out)
  });
  spec.add(cb::ID::SpawnPlayer, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x0c, v);
    out.write_varint(p.pb().ints[0]); // Player's EID
    out.write_uuid(&p.pb().uuids[0]); // Player's UUID
    out.write_fixed_int(p.pb().doubles[0]); // X
    out.write_fixed_int(p.pb().doubles[1]); // Y
    out.write_fixed_int(p.pb().doubles[2]); // Z
    out.write_u8(p.pb().bytes[0]); // Yaw
    out.write_u8(p.pb().bytes[1]); // Pitch
    out.write_u16(0); // Current item (0 -> none)
    out.write_u8(0xff); // Empty Metadata
    Ok(out)
  });
  spec.add(cb::ID::CollectItem, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x0d, v);
    out.write_varint(p.pb().ints[0]); // Collected EID
    out.write_varint(p.pb().ints[1]); // Collector EID
    Ok(out)
  });
  spec.add(cb::ID::ChunkData, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x21, v);
    // TODO: Error handling should be done within the packet.
    let chunk = match p.read_other().unwrap() {
      Other::Chunk(c) => c,
      o => return Err(Error::new(ErrorKind::InvalidData, format!("expected chunk, got {:?}", o))),
    };
    dbg!(&chunk);
    out.write_i32(chunk.x);
    out.write_i32(chunk.z);
    out.write_bool(true); // Always a new chunk

    let biomes = true; // Always true with new chunk set
    let skylight = true; // Assume overworld

    let mut bitmask = 0;
    for (y, _) in &chunk.sections {
      bitmask |= 1 << y;
    }
    out.write_u16(bitmask);

    let mut buf = Buffer::new(vec![]);
    // Makes an ordered list of chunk sections
    let mut sections = vec![None; 16];
    for (y, s) in &chunk.sections {
      sections[*y as usize] = Some(s);
    }
    // Iterates through chunks in order, from ground up
    let mut total_sections = 0;
    for s in sections {
      match s {
        Some(s) => {
          total_sections += 1;
          buf.write_buf(&s.data);
        }
        _ => (),
      }
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

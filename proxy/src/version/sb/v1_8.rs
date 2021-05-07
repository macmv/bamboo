use super::PacketSpec;
use crate::packet::Packet;

use common::{
  net::{sb, Other},
  util::Buffer,
  version::ProtocolVersion,
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: Vec::new() };
  spec.add(0x00, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::KeepAlive);
    out.set_int(0, p.read_varint()); // Keep alive id
    Ok(out)
  });
  spec.add(0x01, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::ChatMessage);
    out.set_str(0, p.read_str()); // The message the client sent in plaintext
    Ok(out)
  });
  spec.add(0x02, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::InteractEntity);
    out.set_int(0, p.read_varint()); // Target EID
    let ty = p.read_varint(); // Type:
                              // 0 -> Interact
                              // 1 -> Attack
                              // 2 -> Interact At
    out.set_int(1, ty);
    if ty == 2 {
      // Type is interact at
      out.set_float(0, p.read_f32()); // Target X
      out.set_float(1, p.read_f32()); // Target Y
      out.set_float(2, p.read_f32()); // Target Z
    }
    Ok(out)
  });
  spec.add(0x03, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerOnGround);
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x04, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerPosition);
    out.set_double(0, p.read_f64()); // Pos X
    out.set_double(1, p.read_f64()); // Feet Y1
    out.set_double(2, p.read_f64()); // Pos Z
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x05, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerRotation);
    out.set_float(0, p.read_f32()); // Yaw
    out.set_float(1, p.read_f32()); // Pitch
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x06, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerPositionAndRotation);
    out.set_double(0, p.read_f64()); // Pos X
    out.set_double(1, p.read_f64()); // Feet Y
    out.set_double(2, p.read_f64()); // Pos Z
    out.set_float(0, p.read_f32()); // Yaw
    out.set_float(1, p.read_f32()); // Pitch
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x07, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerDigging);
    out.set_byte(0, p.read_u8()); // Action:
                                  // 0 -> start digging
                                  // 1 -> cancel digging
                                  // 2 -> finished digging
                                  // 3 -> drop item stack
                                  // 4 -> drop item
                                  // 5 -> shoot arrow / finish eating

    out.set_pos(0, p.read_pos()); // Block pos
    out.set_byte(1, p.read_u8()); // Face
    Ok(out)
  });
  spec.add(0x08, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerBlockPlace);
    out.set_pos(0, p.read_pos()); // Block pos
    out.set_byte(0, p.read_u8()); // Face
                                  // TODO: Parse slot data here
    if p.read_bool() {
      p.read_varint(); // Item id
      p.read_u8(); // Item count
      p.read_u8(); // NBT (lets hope its empty)
    }
    out.set_byte(0, p.read_u8()); // Cursor pos X
    out.set_byte(1, p.read_u8()); // Cursor pos Y
    out.set_byte(2, p.read_u8()); // Cursor pos Z
    Ok(out)
  });
  spec.add(0x17, |mut p: Packet| {
    let mut out = sb::Packet::new(sb::ID::PluginMessage);
    out.set_str(0, p.read_str()); // Plugin channel
    out.set_byte_arr(0, p.read_all()); // Data
    Ok(out)
  });
  spec
}

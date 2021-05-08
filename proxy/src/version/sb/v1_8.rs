use super::PacketSpec;
use crate::packet::Packet;

use common::net::sb;

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: Vec::new() };
  spec.add(0x00, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::KeepAlive);
    out.set_int(0, p.read_varint()); // Keep alive id
    Ok(out)
  });
  spec.add(0x01, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::ChatMessage);
    out.set_str(0, p.read_str()); // The message the client sent in plaintext
    Ok(out)
  });
  spec.add(0x02, |p: &mut Packet| {
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
  spec.add(0x03, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerOnGround);
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x04, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerPosition);
    out.set_double(0, p.read_f64()); // Pos X
    out.set_double(1, p.read_f64()); // Feet Y1
    out.set_double(2, p.read_f64()); // Pos Z
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x05, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerRotation);
    out.set_float(0, p.read_f32()); // Yaw
    out.set_float(1, p.read_f32()); // Pitch
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x06, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerPositionAndRotation);
    out.set_double(0, p.read_f64()); // Pos X
    out.set_double(1, p.read_f64()); // Feet Y
    out.set_double(2, p.read_f64()); // Pos Z
    out.set_float(0, p.read_f32()); // Yaw
    out.set_float(1, p.read_f32()); // Pitch
    out.set_bool(0, p.read_bool()); // On ground
    Ok(out)
  });
  spec.add(0x07, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerDigging);
    // Action:
    // 0 -> start digging
    // 1 -> cancel digging
    // 2 -> finished digging
    // 3 -> drop item stack
    // 4 -> drop item
    // 5 -> shoot arrow / finish eating
    // Newer clients use a varint for this, so it is stored as an int in grpc.
    out.set_int(0, p.read_u8().into());
    out.set_pos(0, p.read_pos()); // Block pos
    out.set_byte(0, p.read_u8()); // Face
    Ok(out)
  });
  spec.add(0x08, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::PlayerBlockPlace);
    out.set_int(0, 0); // Main hand
    out.set_pos(0, p.read_pos()); // Block pos
    out.set_int(1, p.read_u8().into()); // Face
    let id = p.read_i16(); // Item id
    if id != -1 {
      // TODO: Parse slot in one function
      p.read_u8(); // Item count
      p.read_u16(); // Item damage
      p.read_u8(); // NBT (lets hope its empty, because I couldn't be bothered
                   // to parse it)
    }
    out.set_float(0, p.read_u8() as f32 / 256.0); // Cursor pos X
    out.set_float(1, p.read_u8() as f32 / 256.0); // Cursor pos Y
    out.set_float(2, p.read_u8() as f32 / 256.0); // Cursor pos Z
    out.set_bool(0, false); // If the player is inside a block
    Ok(out)
  });
  spec.add(0x17, |p: &mut Packet| {
    let mut out = sb::Packet::new(sb::ID::PluginMessage);
    out.set_str(0, p.read_str()); // Plugin channel
    out.set_byte_arr(0, p.read_all()); // Data
    Ok(out)
  });
  spec
}

use std::collections::HashMap;

use super::PacketSpec;
use crate::packet::Packet;

use common::{net::cb, version::ProtocolVersion};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::KeepAlive, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x00, v);
    out.write_varint(p.pb().ints[0]); // Keep alive id
    out
  });
  spec.add(cb::ID::JoinGame, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x01, v);
    out.write_varint(p.pb().ints[0]); // EID
    out.write_u8(p.pb().bytes[0]); // Gamemode
    out.write_u8(0); // Dimension. TODO: Cross dimension stuff with the new codec system
    out.write_u8(1); // Difficulty
    out.write_u8(0); // Max players (ignored)
    out.write_str("default"); // Level type
    out.write_bool(p.pb().bools[0]); // Reduced debug info
    out
  });
  spec.add(cb::ID::ChatMessage, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x02, v);
    out.write_str(&p.pb().strs[0]); // Message, json encoded
    out.write_u8(p.pb().bytes[0]); // Position 0: chat box, 1: system message, 2: game info (above hotbar)
    out
  });
  spec.add(cb::ID::TimeUpdate, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x03, v);
    out.write_u64(p.pb().longs[0]); // World age
    out.write_u64(p.pb().longs[1]); // Time of day
    out
  });
  spec.add(cb::ID::EntityEquipment, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x04, v);
    out.write_varint(p.pb().ints[0]); // EID
    out.write_i16(p.pb().shorts[0] as i16); // EID
    out.write_u64(p.pb().longs[1]); // Time of day
    out
  });
  spec.add(cb::ID::SpawnPosition, |p: cb::Packet, v: ProtocolVersion| {
    let mut out = Packet::new(0x05, v);
    out.write_pos(p.pb().longs[0]); // The location that your compass points to
    out
  });
  spec
}

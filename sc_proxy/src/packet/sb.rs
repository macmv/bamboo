use super::TypeConverter;
use crate::gnet::sb::Packet as GPacket;
use sc_common::{
  net::sb::Packet,
  util::{nbt::NBT, Buffer, Item},
  version::ProtocolVersion,
};
use std::{error::Error, fmt};

#[derive(Debug, Clone)]
pub struct ReadError {
  packet: GPacket,
  kind:   ReadErrorKind,
}

#[derive(Debug, Clone)]
pub enum ReadErrorKind {
  UnknownPacket,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.kind {
      ReadErrorKind::UnknownPacket => write!(f, "unknown packet {:?}", self.packet),
    }
  }
}

impl Error for ReadError {}

pub trait FromTcp {
  fn from_tcp(p: GPacket, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self, ReadError>
  where
    Self: Sized;
}

impl FromTcp for Packet {
  fn from_tcp(p: GPacket, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self, ReadError> {
    Ok(match p {
      GPacket::ChatV8 { message } | GPacket::ChatV11 { message } => Packet::Chat { msg: message },
      GPacket::CreativeInventoryActionV8 { slot_id, unknown, .. } => {
        let mut buf = Buffer::new(unknown);
        Packet::CreativeInventoryUpdate {
          slot: slot_id.try_into().unwrap(),
          item: read_item(ver, &mut buf, conv),
        }
      }
      GPacket::KeepAliveV8 { key: id } => Packet::KeepAlive { id },
      GPacket::KeepAliveV12 { key: id } => Packet::KeepAlive { id: id as i32 },
      GPacket::PlayerV8 { on_ground, .. } => Packet::PlayerOnGround { on_ground },
      GPacket::PlayerLookV8 { yaw, pitch, on_ground, .. }
      | GPacket::PlayerRotationV9 { yaw, pitch, on_ground, .. } => {
        Packet::PlayerLook { yaw, pitch, on_ground }
      }
      GPacket::PlayerRotationV17 { unknown, .. } => {
        let mut buf = Buffer::new(unknown);
        let yaw = buf.read_f32();
        let pitch = buf.read_f32();
        let on_ground = buf.read_bool();
        Packet::PlayerLook { yaw, pitch, on_ground }
      }
      GPacket::PlayerPosLookV8 { x, y, z, yaw, pitch, on_ground, .. }
      | GPacket::PlayerPositionRotationV9 { x, y, z, yaw, pitch, on_ground, .. } => {
        Packet::PlayerPosLook { x, y, z, yaw, pitch, on_ground }
      }
      GPacket::PlayerPositionRotationV17 { unknown, .. } => {
        let mut buf = Buffer::new(unknown);
        let x = buf.read_f64();
        let y = buf.read_f64();
        let z = buf.read_f64();
        let yaw = buf.read_f32();
        let pitch = buf.read_f32();
        let on_ground = buf.read_bool();
        Packet::PlayerPosLook { x, y, z, yaw, pitch, on_ground }
      }
      GPacket::PlayerPositionV8 { x, y, z, on_ground, .. } => {
        Packet::PlayerPos { x, y, z, on_ground }
      }
      GPacket::PlayerPositionV17 { unknown, .. } => {
        let mut buf = Buffer::new(unknown);
        let x = buf.read_f64();
        let y = buf.read_f64();
        let z = buf.read_f64();
        Packet::PlayerPos { x, y, z, on_ground: false }
      }
      GPacket::UpdatePlayerAbilitiesV14 { flying, .. }
      | GPacket::UpdatePlayerAbilitiesV16 { flying, .. } => Packet::Flying { flying },
      _ => return Err(ReadError { packet: p, kind: ReadErrorKind::UnknownPacket }),
    })
  }
}

fn read_item(ver: ProtocolVersion, buf: &mut Buffer, conv: &TypeConverter) -> Item {
  if ver < ProtocolVersion::V1_13 {
    let id = buf.read_i16();
    let count;
    let damage;
    let nbt;
    if id == -1 {
      count = 0;
      damage = 0;
      nbt = NBT::empty("");
    } else {
      count = buf.read_u8();
      damage = buf.read_i16();
      nbt = buf.read_nbt();
    }
    Item::new(conv.item_to_new(id as u32, ver.block()) as i32, count, damage, nbt)
  } else {
    if buf.read_bool() {
      let id = buf.read_varint();
      let count = buf.read_u8();
      let nbt = buf.read_nbt();
      Item::new(conv.item_to_new(id as u32, ver.block()) as i32, count, 0, nbt)
    } else {
      Item::new(0, 0, 0, NBT::empty(""))
    }
  }
}

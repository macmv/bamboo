use super::TypeConverter;
use crate::{
  gnet::{sb::Packet as GPacket, tcp},
  Error, Result,
};
use sc_common::{
  math::Pos,
  nbt::NBT,
  net::sb::{DigStatus, Packet},
  util::{Buffer, Face, Hand, Item},
  version::ProtocolVersion,
};

pub trait FromTcp {
  fn from_tcp(p: GPacket, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self>
  where
    Self: Sized;
}

impl FromTcp for Packet {
  fn from_tcp(p: GPacket, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    Ok(match p {
      GPacket::ChatV8 { message } | GPacket::ChatV11 { message } => Packet::Chat { msg: message },
      GPacket::CreativeInventoryActionV8 { slot_id, mut unknown, .. } => {
        let mut buf = Buffer::new(&mut unknown);
        Packet::CreativeInventoryUpdate {
          slot: slot_id.try_into().unwrap(),
          item: read_item(ver, &mut buf, conv)?,
        }
      }
      GPacket::KeepAliveV8 { key: id } => Packet::KeepAlive { id },
      GPacket::KeepAliveV12 { key: id } => Packet::KeepAlive { id: id as i32 },
      GPacket::PlayerActionV14 { pos, action, unknown, .. } => {
        let mut buf = tcp::Packet::from_buf_id(unknown, 0, ver);
        if action < 2 {
          Packet::BlockDig {
            pos,
            status: DigStatus::from_id(action as u8),
            face: Face::from_id(buf.read_varint()? as u8),
          }
        } else {
          warn!("need to implement dropping item packets");
          // placeholder
          Packet::UseItem { hand: Hand::Main }
        }
      }
      GPacket::PlayerBlockPlacementV8 { position, placed_block_direction, unknown: _, .. } => {
        // let mut buf = Buffer::new(unknown);
        if position == Pos::new(-1, -1, -1) && placed_block_direction == 255 {
          Packet::UseItem { hand: Hand::Main }
        } else {
          Packet::BlockPlace {
            pos:  position,
            face: Face::from_id(placed_block_direction as u8),
            hand: Hand::Main,
          }
        }
      }
      GPacket::PlayerInteractBlockV9 { position, placed_block_direction, hand, .. } => {
        Packet::BlockPlace {
          pos:  position,
          face: Face::from_id(placed_block_direction as u8),
          hand: Hand::from_id(hand as u8),
        }
      }
      GPacket::PlayerInteractBlockV14 { hand, unknown, .. } => {
        let mut buf = tcp::Packet::from_buf_id(unknown, 0, ver);
        // `unknown` has these fields:
        // - position
        // - face (varint)
        // - cursor x (float)
        // - cursor y (float)
        // - cursor z (float)
        // - inside block (bool)
        Packet::BlockPlace {
          pos:  buf.read_pos()?,
          face: Face::from_id(buf.read_varint()? as u8),
          hand: Hand::from_id(hand as u8),
        }
      }
      GPacket::PlayerInteractItemV9 { hand } => Packet::UseItem { hand: Hand::from_id(hand as u8) },
      GPacket::PlayerV8 { on_ground, .. } => Packet::PlayerOnGround { on_ground },
      GPacket::PlayerLookV8 { yaw, pitch, on_ground, .. }
      | GPacket::PlayerRotationV9 { yaw, pitch, on_ground, .. } => {
        Packet::PlayerLook { yaw, pitch, on_ground }
      }
      GPacket::PlayerRotationV17 { mut unknown, .. } => {
        let mut buf = Buffer::new(&mut unknown);
        let yaw = buf.read_f32()?;
        let pitch = buf.read_f32()?;
        let on_ground = buf.read_bool()?;
        Packet::PlayerLook { yaw, pitch, on_ground }
      }
      GPacket::PlayerPosLookV8 { x, y, z, yaw, pitch, on_ground, .. }
      | GPacket::PlayerPositionRotationV9 { x, y, z, yaw, pitch, on_ground, .. } => {
        Packet::PlayerPosLook { x, y, z, yaw, pitch, on_ground }
      }
      GPacket::PlayerPositionRotationV17 { mut unknown, .. } => {
        let mut buf = Buffer::new(&mut unknown);
        let x = buf.read_f64()?;
        let y = buf.read_f64()?;
        let z = buf.read_f64()?;
        let yaw = buf.read_f32()?;
        let pitch = buf.read_f32()?;
        let on_ground = buf.read_bool()?;
        Packet::PlayerPosLook { x, y, z, yaw, pitch, on_ground }
      }
      GPacket::PlayerPositionV8 { x, y, z, on_ground, .. } => {
        Packet::PlayerPos { x, y, z, on_ground }
      }
      GPacket::PlayerPositionV17 { mut unknown, .. } => {
        let mut buf = Buffer::new(&mut unknown);
        let x = buf.read_f64()?;
        let y = buf.read_f64()?;
        let z = buf.read_f64()?;
        Packet::PlayerPos { x, y, z, on_ground: false }
      }
      GPacket::UpdatePlayerAbilitiesV14 { flying, .. }
      | GPacket::UpdatePlayerAbilitiesV16 { flying, .. } => Packet::Flying { flying },
      gpacket => return Err(Error::UnknownSB(Box::new(gpacket))),
    })
  }
}

fn read_item(ver: ProtocolVersion, buf: &mut Buffer, conv: &TypeConverter) -> Result<Item> {
  Ok(if ver < ProtocolVersion::V1_13 {
    let id = buf.read_i16()?;
    let count;
    let damage;
    let nbt;
    if id == -1 {
      count = 0;
      damage = 0;
      nbt = NBT::empty("");
    } else {
      count = buf.read_u8()?;
      damage = buf.read_i16()?;
      nbt = buf.read_nbt()?;
    }
    Item::new(conv.item_to_new(id as u32, ver.block()) as i32, count, damage, nbt)
  } else {
    if buf.read_bool()? {
      let id = buf.read_varint()?;
      let count = buf.read_u8()?;
      let nbt = buf.read_nbt()?;
      Item::new(conv.item_to_new(id as u32, ver.block()) as i32, count, 0, nbt)
    } else {
      Item::new(0, 0, 0, NBT::empty(""))
    }
  })
}

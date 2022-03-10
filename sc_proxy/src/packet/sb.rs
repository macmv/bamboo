use super::TypeConverter;
use crate::{
  gnet::{sb::Packet as GPacket, tcp},
  Error, Result,
};
use sc_common::{
  math::Pos,
  net::sb::{Button, ClickWindow, DigStatus, Packet},
  util::{Buffer, Face, Hand},
  version::ProtocolVersion,
};
use std::{io, io::ErrorKind};

pub trait FromTcp {
  fn from_tcp(p: GPacket, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self>
  where
    Self: Sized;
}

impl FromTcp for Packet {
  fn from_tcp(p: GPacket, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    Ok(match p {
      GPacket::ChatV8 { message } | GPacket::ChatV11 { message } => Packet::Chat { msg: message },
      GPacket::ClickWindowV8 {
        window_id,
        slot_id,
        used_button,
        action_number,
        clicked_item: _,
        mode,
        unknown,
      } => {
        let mut buf = tcp::Packet::from_buf_id(unknown, 0, ver);
        let _item = buf.read_item(&conv)?;
        Packet::ClickWindow {
          id:   window_id.try_into().unwrap(),
          slot: slot_id.try_into().unwrap(),
          mode: click_window(mode, used_button)?,
        }
      }
      GPacket::CloseWindowV8 { window_id } => {
        Packet::WindowClose { wid: window_id.try_into().unwrap() }
      }
      GPacket::ClickWindowV9 {
        window_id,
        slot_id,
        used_button,
        action_number,
        clicked_item: _,
        mode,
        unknown,
      } => {
        let mut buf = tcp::Packet::from_buf_id(unknown, 0, ver);
        let _item = buf.read_item(&conv)?;
        Packet::ClickWindow {
          id:   window_id.try_into().unwrap(),
          slot: slot_id.try_into().unwrap(),
          mode: click_window(mode, used_button)?,
        }
      }
      GPacket::CreativeInventoryActionV8 { slot_id, unknown, .. } => {
        let mut buf = tcp::Packet::from_buf_id(unknown, 0, ver);
        Packet::CreativeInventoryUpdate {
          slot: slot_id.try_into().unwrap(),
          item: buf.read_item(conv)?,
        }
      }
      GPacket::HeldItemChangeV8 { slot_id } => Packet::ChangeHeldItem { slot: slot_id as u8 },
      GPacket::KeepAliveV8 { key: id } => Packet::KeepAlive { id },
      GPacket::KeepAliveV12 { key: id } => Packet::KeepAlive { id: id as i32 },
      GPacket::PlayerActionV14 { pos, action, unknown, .. } => {
        let mut buf = tcp::Packet::from_buf_id(unknown, 0, ver);
        if action <= 2 {
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
      GPacket::PlayerDiggingV8 { position, facing: _, status, unknown } => {
        let mut buf = Buffer::new(unknown);
        if status <= 2 {
          Packet::BlockDig {
            pos:    position,
            status: DigStatus::from_id(status as u8),
            face:   Face::from_id(buf.read_varint()? as u8),
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
      GPacket::PlayerInteractBlockV11 { position, placed_block_direction, hand, .. } => {
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

fn click_window(mode: i32, bt: i32) -> Result<ClickWindow> {
  Ok(match mode {
    // Click
    0 => ClickWindow::Click(button(bt)?),
    // Shift click
    1 => ClickWindow::ShiftClick(button(bt)?),
    // Number (puts the item in the given slot in the hotbar)
    2 => ClickWindow::Number(bt.try_into().unwrap()),
    // Middle click
    3 => ClickWindow::Click(Button::Middle),
    // Drop item
    4 => match bt {
      0 => ClickWindow::Drop,
      1 => ClickWindow::DropAll,
      _ => {
        return Err(io::Error::new(ErrorKind::Other, "invalid button for drop item action").into())
      }
    },
    // Drag item
    5 => match bt {
      0 | 4 | 8 => ClickWindow::DragStart(button(bt)?),
      1 | 5 | 9 => ClickWindow::DragAdd(button(bt)?),
      2 | 6 | 10 => ClickWindow::DragEnd(button(bt)?),
      _ => {
        return Err(io::Error::new(ErrorKind::Other, "invalid button for drag item action").into())
      }
    },
    // Double click
    6 => ClickWindow::DoubleClick,
    _ => return Err(io::Error::new(ErrorKind::Other, "invalid click window mode").into()),
  })
}

fn button(bt: i32) -> Result<Button> {
  Ok(match bt {
    0 => Button::Left,
    1 => Button::Right,
    2 => Button::Middle,
    _ => return Err(io::Error::new(ErrorKind::Other, "invalid button").into()),
  })
}

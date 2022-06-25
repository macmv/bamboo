use super::TypeConverter;
use crate::{
  gnet::{
    sb::{packet as gpacket, Packet as GPacket},
    tcp,
  },
  Error, Result,
};
use bb_common::{
  math::{FPos, Pos},
  net::sb::{Button, ClickWindow, DigStatus, Packet, PlayerCommand, UseEntityAction},
  util::{Face, Hand},
  version::ProtocolVersion,
};
use std::{io, io::ErrorKind};

pub trait FromTcp<G> {
  fn from_tcp(p: G, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self>
  where
    Self: Sized;
}

impl FromTcp<GPacket> for Packet {
  fn from_tcp(p: GPacket, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    match p {
      GPacket::Animation(g) => Packet::from_tcp(g, ver, conv),
      GPacket::Chat(g) => Packet::from_tcp(g, ver, conv),
      GPacket::CommandExecution(g) => Packet::from_tcp(g, ver, conv),
      GPacket::ClickWindow(g) => Packet::from_tcp(g, ver, conv),
      GPacket::CloseWindow(g) => Packet::from_tcp(g, ver, conv),
      GPacket::CloseHandledScreen(g) => Packet::from_tcp(g, ver, conv),
      GPacket::ClickSlot(g) => Packet::from_tcp(g, ver, conv),
      GPacket::CreativeInventoryAction(g) => Packet::from_tcp(g, ver, conv),
      GPacket::HeldItemChange(g) => Packet::from_tcp(g, ver, conv),
      GPacket::KeepAlive(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerDig(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerBlockPlacement(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerInteractBlock(g) => Packet::from_tcp(g, ver, conv),
      GPacket::UseEntity(g) => Packet::from_tcp(g, ver, conv),
      gpacket => Err(Error::UnknownSB(Box::new(gpacket))),
    }
  }
}

impl FromTcp<gpacket::Animation> for Packet {
  fn from_tcp(p: gpacket::Animation, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::Animation::*;
    Ok(match p {
      V8(g) => Packet::Animation { hand: Hand::Main },
      V9(g) => Packet::Animation { hand: Hand::from_id(g.hand as u8) },
    })
  }
}
impl FromTcp<gpacket::Chat> for Packet {
  fn from_tcp(p: gpacket::Chat, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::Chat::*;
    Ok(match p {
      V8(g) => Packet::Chat { msg: g.message },
      V11(g) => Packet::Chat { msg: g.message },
      V19(g) => Packet::Chat { msg: g.chat_message },
    })
  }
}
impl FromTcp<gpacket::CommandExecution> for Packet {
  fn from_tcp(
    p: gpacket::CommandExecution,
    ver: ProtocolVersion,
    conv: &TypeConverter,
  ) -> Result<Self> {
    use gpacket::CommandExecution::*;
    Ok(match p {
      V19(g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        let msg = buf.read_str(256)?;
        let _time = buf.read_u64()?;
        let _salt = buf.read_u64()?;
        let _args_map_len = buf.read_varint()?;
        let _signed = buf.read_bool()?;
        Packet::Chat { msg: format!("/{msg}") }
      }
    })
  }
}
impl FromTcp<gpacket::ClickWindow> for Packet {
  fn from_tcp(p: gpacket::ClickWindow, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::ClickWindow::*;
    Ok(match p {
      V8(mut g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        let _item = buf.read_item(conv)?;
        if g.slot_id == -1 {
          g.slot_id = -999;
        }
        Packet::ClickWindow {
          wid:  g.window_id.try_into().unwrap(),
          slot: g.slot_id.try_into().unwrap(),
          mode: click_window(g.mode, g.used_button)?,
        }
      }
      V9(mut g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        let _item = buf.read_item(conv)?;
        if g.slot_id == -1 {
          g.slot_id = -999;
        }
        Packet::ClickWindow {
          wid:  g.window_id.try_into().unwrap(),
          slot: g.slot_id.try_into().unwrap(),
          mode: click_window(g.mode, g.used_button)?,
        }
      }
    })
  }
}
impl FromTcp<gpacket::CloseWindow> for Packet {
  fn from_tcp(p: gpacket::CloseWindow, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::CloseWindow::*;
    Ok(match p {
      V8(g) => Packet::WindowClose { wid: g.window_id.try_into().unwrap() },
    })
  }
}
impl FromTcp<gpacket::CloseHandledScreen> for Packet {
  fn from_tcp(
    p: gpacket::CloseHandledScreen,
    ver: ProtocolVersion,
    conv: &TypeConverter,
  ) -> Result<Self> {
    use gpacket::CloseHandledScreen::*;
    Ok(match p {
      V16(g) => Packet::WindowClose { wid: g.sync_id.try_into().unwrap() },
    })
  }
}
impl FromTcp<gpacket::ClickSlot> for Packet {
  fn from_tcp(p: gpacket::ClickSlot, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::ClickSlot::*;
    Ok(match p {
      V16(mut g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        let slots = buf.read_varint()?;
        for _ in 0..slots {
          let _slot = buf.read_u16()?;
          let _item = buf.read_item(conv)?;
        }
        let _item = buf.read_item(conv)?;
        if g.slot == -1 {
          g.slot = -999;
        }
        Packet::ClickWindow {
          wid:  g.sync_id.try_into().unwrap(),
          slot: g.slot.try_into().unwrap(),
          mode: click_window(g.action_type, g.button)?,
        }
      }
      V17(mut g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        let slots = buf.read_varint()?;
        for _ in 0..slots {
          let _slot = buf.read_u16()?;
          let _item = buf.read_item(conv)?;
        }
        let _item = buf.read_item(conv)?;
        if g.slot == -1 {
          g.slot = -999;
        }
        Packet::ClickWindow {
          wid:  g.sync_id.try_into().unwrap(),
          slot: g.slot.try_into().unwrap(),
          mode: click_window(g.action_type, g.button)?,
        }
      }
    })
  }
}
impl FromTcp<gpacket::CreativeInventoryAction> for Packet {
  fn from_tcp(
    p: gpacket::CreativeInventoryAction,
    ver: ProtocolVersion,
    conv: &TypeConverter,
  ) -> Result<Self> {
    use gpacket::CreativeInventoryAction::*;
    Ok(match p {
      V8(g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        Packet::CreativeInventoryUpdate {
          slot: g.slot_id.try_into().unwrap(),
          item: buf.read_item(conv)?,
        }
      }
    })
  }
}
impl FromTcp<gpacket::HeldItemChange> for Packet {
  fn from_tcp(
    p: gpacket::HeldItemChange,
    ver: ProtocolVersion,
    conv: &TypeConverter,
  ) -> Result<Self> {
    use gpacket::HeldItemChange::*;
    Ok(match p {
      V8(g) => Packet::ChangeHeldItem { slot: g.slot_id as u8 },
    })
  }
}
impl FromTcp<gpacket::KeepAlive> for Packet {
  fn from_tcp(p: gpacket::KeepAlive, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::KeepAlive::*;
    Ok(match p {
      V8(g) => Packet::KeepAlive { id: g.key },
      V12(g) => Packet::KeepAlive { id: g.key as i32 },
    })
  }
}
impl FromTcp<gpacket::PlayerDig> for Packet {
  fn from_tcp(p: gpacket::PlayerDig, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::PlayerDig::*;
    Ok(match p {
      V8(g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        match g.status {
          0 | 1 | 2 => Packet::BlockDig {
            pos:    g.position,
            status: DigStatus::from_id(g.status as u8),
            face:   Face::from_id(buf.read_varint()? as u8),
          },
          3 => Packet::ClickWindow { wid: u8::MAX, slot: 0, mode: ClickWindow::DropAll },
          4 => Packet::ClickWindow { wid: u8::MAX, slot: 0, mode: ClickWindow::Drop },
          5 => {
            return Err(io::Error::new(ErrorKind::Other, "need to implement eating packet").into())
          }
          6 => {
            return Err(
              io::Error::new(ErrorKind::Other, "need to implement swap item packet").into(),
            )
          }
          _ => return Err(io::Error::new(ErrorKind::Other, "invalid player dig action").into()),
        }
      }
    })
  }
}
impl FromTcp<gpacket::PlayerBlockPlacement> for Packet {
  fn from_tcp(
    p: gpacket::PlayerBlockPlacement,
    ver: ProtocolVersion,
    conv: &TypeConverter,
  ) -> Result<Self> {
    use gpacket::PlayerBlockPlacement::*;
    Ok(match p {
      V8(g) => {
        // let mut buf = Buffer::new(unknown);
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        if g.position == Pos::new(-1, -1, -1) && g.placed_block_direction == 255 {
          Packet::UseItem { hand: Hand::Main }
        } else {
          let _slot = buf.read_item(conv)?;
          let cursor_x = buf.read_u8()?;
          let cursor_y = buf.read_u8()?;
          let cursor_z = buf.read_u8()?;
          Packet::BlockPlace {
            pos:    g.position,
            face:   Face::from_id(g.placed_block_direction as u8),
            hand:   Hand::Main,
            cursor: FPos::new(
              cursor_x as f64 / 128.0,
              cursor_y as f64 / 128.0,
              cursor_z as f64 / 128.0,
            ),
          }
        }
      }
    })
  }
}
impl FromTcp<gpacket::PlayerInteractBlock> for Packet {
  fn from_tcp(
    p: gpacket::PlayerInteractBlock,
    ver: ProtocolVersion,
    conv: &TypeConverter,
  ) -> Result<Self> {
    use gpacket::PlayerInteractBlock::*;
    Ok(match p {
      V9(g) => Packet::BlockPlace {
        pos:    g.position,
        face:   Face::from_id(g.placed_block_direction as u8),
        hand:   Hand::from_id(g.hand as u8),
        cursor: FPos::new(g.facing_x.into(), g.facing_y.into(), g.facing_z.into()),
      },
      V11(g) => Packet::BlockPlace {
        pos:    g.position,
        face:   Face::from_id(g.placed_block_direction as u8),
        hand:   Hand::from_id(g.hand as u8),
        cursor: FPos::new(g.facing_x.into(), g.facing_y.into(), g.facing_z.into()),
      },
      V14(g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        let pos = buf.read_pos()?;
        let face = Face::from_id(buf.read_varint()? as u8);
        let cursor =
          FPos::new(buf.read_f32()?.into(), buf.read_f32()?.into(), buf.read_f32()?.into());
        let _in_head = buf.read_bool()?;
        let _sequence = if ver >= ProtocolVersion::V1_19 { Some(buf.read_varint()?) } else { None };
        Packet::BlockPlace { hand: Hand::from_id(g.hand as u8), pos, face, cursor }
      }
    })
  }
}
impl FromTcp<gpacket::UseEntity> for Packet {
  fn from_tcp(p: gpacket::UseEntity, ver: ProtocolVersion, conv: &TypeConverter) -> Result<Self> {
    use gpacket::UseEntity::*;
    Ok(match p {
      V8(g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        Packet::UseEntity {
          eid:      g.entity_id,
          action:   match g.action {
            0 => UseEntityAction::Interact(if ver == ProtocolVersion::V1_8 {
              Hand::Main
            } else {
              Hand::from_id(buf.read_u8()?)
            }),
            1 => UseEntityAction::Attack,
            2 => UseEntityAction::InteractAt(
              FPos::new(buf.read_f32()?.into(), buf.read_f32()?.into(), buf.read_f32()?.into()),
              if ver == ProtocolVersion::V1_8 { Hand::Main } else { Hand::from_id(buf.read_u8()?) },
            ),
            _ => return Err(io::Error::new(ErrorKind::Other, "invalid use entity action").into()),
          },
          sneaking: match ver >= ProtocolVersion::V1_16_5 {
            true => Some(buf.read_bool()?),
            false => None,
          },
        }
      }
      V17(g) => {
        let mut buf = tcp::Packet::from_buf_id(g.unknown, 0, ver);
        Packet::UseEntity {
          eid:      g.entity_id,
          action:   match g.v_2 {
            0 => UseEntityAction::Interact(Hand::from_id(buf.read_u8()?)),
            1 => UseEntityAction::Attack,
            2 => UseEntityAction::InteractAt(
              FPos::new(buf.read_f32()?.into(), buf.read_f32()?.into(), buf.read_f32()?.into()),
              if ver == ProtocolVersion::V1_8 { Hand::Main } else { Hand::from_id(buf.read_u8()?) },
            ),
            _ => return Err(io::Error::new(ErrorKind::Other, "invalid use entity action").into()),
          },
          sneaking: Some(buf.read_bool()?),
        }
      }
    })
  }
}
/*
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerCommandV8 { entity_id: _, action, aux_data: _ } => Packet::PlayerCommand {
        command: match action {
          0 => PlayerCommand::StartSneak,
          1 => PlayerCommand::StopSneak,
          2 => PlayerCommand::LeaveBed,
          3 => PlayerCommand::StartSprint,
          4 => PlayerCommand::StopSprint,
          _ => return Err(io::Error::new(ErrorKind::Other, "invalid player action").into()),
        },
      },
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerInteractItemV9 { hand } => Packet::UseItem { hand: Hand::from_id(hand as u8) },
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerV8 { on_ground, .. } | GPacket::PlayerOnGroundV14 { on_ground } => {
        Packet::PlayerOnGround { on_ground }
      }
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerOnGroundV17 { v_1, unknown: _ } => Packet::PlayerOnGround { on_ground: v_1 },
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerLookV8 { yaw, pitch, on_ground, .. }
      | GPacket::PlayerRotationV9 { yaw, pitch, on_ground, .. } => {
        Packet::PlayerLook { yaw, pitch, on_ground }
      }
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerRotationV17 { unknown: _, v_1, v_2, v_3 } => {
        Packet::PlayerLook { yaw: v_1, pitch: v_2, on_ground: v_3 }
      }
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerPosLookV8 { x, y, z, yaw, pitch, on_ground, .. }
      | GPacket::PlayerPositionRotationV9 { x, y, z, yaw, pitch, on_ground, .. } => {
        Packet::PlayerPosLook { x, y, z, yaw, pitch, on_ground }
      }
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerPositionRotationV17 { unknown: _, v_1, v_3, v_5, v_7, v_8, v_9 } => {
        Packet::PlayerPosLook {
          x:         v_1,
          y:         v_3,
          z:         v_5,
          yaw:       v_7,
          pitch:     v_8,
          on_ground: v_9,
        }
      }
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerPositionV8 { x, y, z, on_ground, .. } => {
        Packet::PlayerPos { x, y, z, on_ground }
      }
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerPositionV17 { unknown: _, v_1, v_3, v_5, v_7 } => {
        Packet::PlayerPos { x: v_1, y: v_3, z: v_5, on_ground: v_7 }
      }
  }
}
impl FromTcp<> for Packet {
  fn from_tcp(p: gpacket::Chat) -> Result<Self> {
      GPacket::PlayerAbilitiesV8 { flying, .. }
      | GPacket::UpdatePlayerAbilitiesV14 { flying, .. }
      | GPacket::UpdatePlayerAbilitiesV16 { flying, .. } => Packet::Flying { flying },
  }
}
*/

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
      0 | 4 | 8 => ClickWindow::DragStart(button(bt / 4)?),
      1 | 5 | 9 => ClickWindow::DragAdd(button(bt / 4)?),
      2 | 6 | 10 => ClickWindow::DragEnd(button(bt / 4)?),
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
    _ => return Err(io::Error::new(ErrorKind::Other, format!("invalid button {bt}")).into()),
  })
}

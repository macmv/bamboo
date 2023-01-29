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
      GPacket::PlayerCommand(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerInteractItem(g) => Packet::from_tcp(g, ver, conv),
      GPacket::Player(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerOnGround(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerLook(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerRotation(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerPosLook(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerPositionRotation(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerPosition(g) => Packet::from_tcp(g, ver, conv),
      GPacket::PlayerAbilities(g) => Packet::from_tcp(g, ver, conv),
      GPacket::UpdatePlayerAbilities(g) => Packet::from_tcp(g, ver, conv),
      gpacket => Err(Error::UnknownSB(Box::new(gpacket))),
    }
  }
}

macro_rules! from_tcp {
  (
    $packet:ident, $ver:ident, $conv:ident,
    {
      $( $match:ident($match_var:pat) $( $buf:ident = $unknown:expr )? => $value:expr, )*
    }
  ) => {
    impl FromTcp<gpacket::$packet> for Packet {
      fn from_tcp(p: gpacket::$packet, $ver: ProtocolVersion, $conv: &TypeConverter) -> Result<Self> {
        let wrapped = GPacket::$packet(p);
        let _tcp_id = wrapped.tcp_id($ver) as i32;
        let p = match wrapped { GPacket::$packet(p) => p, _ => unreachable!() };
        Ok(match p {
          $(
            gpacket::$packet::$match($match_var) => {
              $( let mut $buf = tcp::Packet::from_buf_id($unknown, _tcp_id, $ver); )?
              $value
            }
          )*
        })
      }
    }
  }
}

from_tcp!(Animation, _ver, _conv, {
  V8(_g) => Packet::Animation { hand: Hand::Main },
  V9(g) => Packet::Animation { hand: Hand::from_id(g.hand as u8) },
});
from_tcp!(Chat, _ver, _conv, {
  V8(g) => Packet::Chat { msg: g.message },
  V11(g) => Packet::Chat { msg: g.message },
  V19(g) => Packet::Chat { msg: String::from_utf8_lossy(&g.unknown).into() },
});
from_tcp!(CommandExecution, ver, _conv, {
  V19(g) buf = g.unknown => {
    let msg = buf.read_str(256)?;
    let _time = buf.read_u64()?;
    let _salt = buf.read_u64()?;
    let _args_map_len = buf.read_varint()?;
    let _signed = buf.read_bool()?;
    Packet::Chat { msg: format!("/{msg}") }
  },
});
from_tcp!(ClickWindow, ver, conv, {
  V8(mut g) buf = g.unknown => {
    let _item = buf.read_item(conv)?;
    if g.slot_id == -1 {
      g.slot_id = -999;
    }
    Packet::ClickWindow {
      wid:  g.window_id.try_into().unwrap(),
      slot: g.slot_id.try_into().unwrap(),
      mode: click_window(g.mode, g.used_button)?,
    }
  },
  V9(mut g) buf = g.unknown => {
    let _item = buf.read_item(conv)?;
    if g.slot_id == -1 {
      g.slot_id = -999;
    }
    Packet::ClickWindow {
      wid:  g.window_id.try_into().unwrap(),
      slot: g.slot_id.try_into().unwrap(),
      mode: click_window(g.mode, g.used_button)?,
    }
  },
});
from_tcp!(CloseWindow, _ver, _conv, {
  V8(g) => Packet::WindowClose { wid: g.window_id.try_into().unwrap() },
});
from_tcp!(CloseHandledScreen, _ver, _conv, {
  V16(g) => Packet::WindowClose { wid: g.sync_id.try_into().unwrap() },
});
from_tcp!(ClickSlot, ver, conv, {
  V16(mut g) buf = g.unknown => {
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
  },
  V17(mut g) buf = g.unknown => {
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
  },
});
from_tcp!(CreativeInventoryAction, ver, conv, {
  V8(g) buf = g.unknown => Packet::CreativeInventoryUpdate {
    slot: g.slot_id.try_into().unwrap(),
    item: buf.read_item(conv)?,
  },
});
from_tcp!(HeldItemChange, _ver, _conv, {
  V8(g) => Packet::ChangeHeldItem { slot: g.slot_id as u8 },
});
from_tcp!(KeepAlive, _ver, _conv, {
  V8(g) => Packet::KeepAlive { id: g.key },
  V12(g) => Packet::KeepAlive { id: g.key as i32 },
});
from_tcp!(PlayerDig, ver, _conv, {
  V8(g) buf = g.unknown => match g.status {
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
  },
});
from_tcp!(PlayerBlockPlacement, ver, conv, {
  V8(g) buf = g.unknown => {
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
          cursor_x as f64 / 16.0,
          cursor_y as f64 / 16.0,
          cursor_z as f64 / 16.0,
        ),
      }
    }
  },
});
from_tcp!(PlayerInteractBlock, ver, _conv, {
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
  V14(g) buf = g.unknown => {
    let pos = buf.read_pos()?;
    let face = Face::from_id(buf.read_varint()? as u8);
    let cursor =
      FPos::new(buf.read_f32()?.into(), buf.read_f32()?.into(), buf.read_f32()?.into());
    let _in_head = buf.read_bool()?;
    let _sequence = if ver >= ProtocolVersion::V1_19 { Some(buf.read_varint()?) } else { None };
    Packet::BlockPlace { hand: Hand::from_id(g.hand as u8), pos, face, cursor }
  },
});
from_tcp!(UseEntity, ver, _conv, {
  V8(g) buf = g.unknown => Packet::UseEntity {
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
  },
  V17(g) buf = g.unknown => Packet::UseEntity {
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
  },
});
from_tcp!(PlayerCommand, _ver, _conv, {
  V8(g) => Packet::PlayerCommand {
    command: match g.action {
      0 => PlayerCommand::StartSneak,
      1 => PlayerCommand::StopSneak,
      2 => PlayerCommand::LeaveBed,
      3 => PlayerCommand::StartSprint,
      4 => PlayerCommand::StopSprint,
      _ => return Err(io::Error::new(ErrorKind::Other, "invalid player action").into()),
    },
  },
});
from_tcp!(PlayerInteractItem, _ver, _conv, {
  V9(g) => Packet::UseItem { hand: Hand::from_id(g.hand as u8) },
  // TODO: Use `g.sequence`.
  V19(g) => Packet::UseItem { hand: Hand::from_id(g.hand as u8) },
});
from_tcp!(Player, _ver, _conv, {
  V8(g) => Packet::PlayerOnGround { on_ground: g.on_ground },
});
from_tcp!(PlayerOnGround, _ver, _conv, {
  V14(g) => Packet::PlayerOnGround { on_ground: g.on_ground },
  V17(g) => Packet::PlayerOnGround { on_ground: g.v_1 },
});
from_tcp!(PlayerLook, _ver, _conv, {
  V8(g) => Packet::PlayerLook { yaw: g.yaw, pitch: g.pitch, on_ground: g.on_ground },
});
from_tcp!(PlayerRotation, _ver, _conv, {
  V9(g) => Packet::PlayerLook { yaw: g.yaw, pitch: g.pitch, on_ground: g.on_ground },
  V17(g) => Packet::PlayerLook { yaw: g.v_1, pitch: g.v_2, on_ground: g.v_3 },
});
from_tcp!(PlayerPosLook, _ver, _conv, {
  V8(g) => Packet::PlayerPosLook {
    x: g.x,
    y: g.y,
    z: g.z,
    yaw: g.yaw,
    pitch: g.pitch,
    on_ground: g.on_ground,
  },
});
from_tcp!(PlayerPositionRotation, _ver, _conv, {
  V9(g) => Packet::PlayerPosLook {
    x: g.x,
    y: g.y,
    z: g.z,
    yaw: g.yaw,
    pitch: g.pitch,
    on_ground: g.on_ground,
  },
  V17(g) => Packet::PlayerPosLook {
    x:         g.v_1,
    y:         g.v_3,
    z:         g.v_5,
    yaw:       g.v_7,
    pitch:     g.v_8,
    on_ground: g.v_9,
  },
});
from_tcp!(PlayerPosition, _ver, _conv, {
  V8(g) => Packet::PlayerPos { x: g.x, y: g.y, z: g.z, on_ground: g.on_ground },
  V17(g) => Packet::PlayerPos { x: g.v_1, y: g.v_3, z: g.v_5, on_ground: g.v_7 },
});
from_tcp!(PlayerAbilities, _ver, _conv, {
  V8(g) => Packet::Flying { flying: g.flying },
});
from_tcp!(UpdatePlayerAbilities, _ver, _conv, {
  V14(g) => Packet::Flying { flying: g.flying },
  V16(g) => Packet::Flying { flying: g.flying },
});

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

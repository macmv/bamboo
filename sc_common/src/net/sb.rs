use crate::{
  math::Pos,
  util::{Face, Hand, Item},
};

#[derive(Debug, Clone, sc_macros::Transfer)]
#[non_exhaustive]
pub enum Packet {
  BlockDig {
    pos:    Pos,
    status: DigStatus,
    face:   Face,
  },
  BlockPlace {
    pos:  Pos,
    face: Face,
    hand: Hand,
  },
  CreativeInventoryUpdate {
    slot: i8,
    item: Item,
  },
  ChangeHeldItem {
    slot: u8,
  },
  Chat {
    msg: String,
  },
  Flying {
    flying: bool,
  },
  KeepAlive {
    id: i32,
  },
  PlayerOnGround {
    on_ground: bool,
  },
  PlayerLook {
    yaw:       f32,
    pitch:     f32,
    on_ground: bool,
  },
  PlayerPos {
    x:         f64,
    y:         f64,
    z:         f64,
    on_ground: bool,
  },
  PlayerPosLook {
    x:         f64,
    y:         f64,
    z:         f64,
    yaw:       f32,
    pitch:     f32,
    on_ground: bool,
  },
  PluginMessage {
    channel: String,
    data:    Vec<u8>,
  },
  UseItem {
    hand: Hand,
  },
}

#[derive(Debug, Clone, sc_macros::Transfer)]
pub enum DigStatus {
  Start,
  Cancel,
  Finish,
}

impl DigStatus {
  pub fn from_id(id: u8) -> Self {
    match id {
      0 => Self::Start,
      1 => Self::Cancel,
      2 => Self::Finish,
      _ => panic!("invalid dig status: {}", id),
    }
  }
}

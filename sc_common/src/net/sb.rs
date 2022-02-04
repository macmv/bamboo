use crate::{
  math::Pos,
  util::{Face, Item},
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
    hand: u8,
  },
  CreativeInventoryUpdate {
    slot: u8,
    item: Item,
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
}

#[derive(Debug, Clone, sc_macros::Transfer)]
pub enum DigStatus {
  Start,
  Cancel,
  Finish,
}

use crate::{
  math::Pos,
  util::{Face, Hand, Item},
};

#[sc_macros::transfer]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Packet {
  #[id = 0]
  BlockDig { pos: Pos, status: DigStatus, face: Face },
  #[id = 1]
  BlockPlace { pos: Pos, face: Face, hand: Hand },
  #[id = 2]
  CreativeInventoryUpdate { slot: i8, item: Item },
  #[id = 3]
  ChangeHeldItem { slot: u8 },
  #[id = 4]
  Chat { msg: String },
  #[id = 5]
  Flying { flying: bool },
  #[id = 6]
  KeepAlive { id: i32 },
  #[id = 7]
  PlayerOnGround { on_ground: bool },
  #[id = 8]
  PlayerLook { yaw: f32, pitch: f32, on_ground: bool },
  #[id = 9]
  PlayerPos { x: f64, y: f64, z: f64, on_ground: bool },
  #[id = 10]
  PlayerPosLook {
    x:         f64,
    y:         f64,
    z:         f64,
    yaw:       f32,
    pitch:     f32,
    on_ground: bool,
  },
  #[id = 11]
  PluginMessage { channel: String, data: Vec<u8> },
  #[id = 12]
  UseItem { hand: Hand },
}

#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub enum DigStatus {
  #[id = 0]
  Start,
  #[id = 1]
  Cancel,
  #[id = 2]
  Finish,
}

impl Default for DigStatus {
  fn default() -> Self { DigStatus::Start }
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

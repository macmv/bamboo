use crate::{
  math::{FPos, Pos},
  util::{Face, Hand, Item},
};
use bb_macros::Transfer;

#[derive(Transfer, Debug, Clone)]
#[non_exhaustive]
pub enum Packet {
  #[id = 17]
  Animation { hand: Hand },
  #[id = 0]
  BlockDig { pos: Pos, status: DigStatus, face: Face },
  #[id = 1]
  BlockPlace { pos: Pos, face: Face, hand: Hand, cursor: FPos },
  #[id = 2]
  CreativeInventoryUpdate { slot: i8, item: Item },
  #[id = 3]
  ChangeHeldItem { slot: u8 },
  #[id = 4]
  Chat { msg: String },
  #[id = 13]
  ClickWindow {
    wid:  u8,
    slot: i16,
    #[must_exist]
    mode: ClickWindow,
  },
  #[id = 5]
  Flying { flying: bool },
  #[id = 6]
  KeepAlive { id: i32 },
  #[id = 16]
  PlayerCommand { command: PlayerCommand },
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
  /// Sneaking will not be present on 1.8-1.15 clients. It should be used if it
  /// is present (it will produce more accurate results for shift-clicking on an
  /// entity).
  #[id = 15]
  UseEntity { eid: i32, action: UseEntityAction, sneaking: Option<bool> },
  #[id = 14]
  WindowClose { wid: u8 },
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum DigStatus {
  #[id = 0]
  #[default]
  Start,
  #[id = 1]
  Cancel,
  #[id = 2]
  Finish,
}

#[derive(Transfer, Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum UseEntityAction {
  #[id = 0]
  #[default]
  Attack,
  #[id = 1]
  Interact(Hand),
  #[id = 2]
  InteractAt(FPos, Hand),
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum PlayerCommand {
  #[id = 0]
  #[default]
  StartSneak,
  #[id = 1]
  StopSneak,
  #[id = 2]
  StartSprint,
  #[id = 3]
  StopSprint,
  #[id = 4]
  LeaveBed,
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

// See https://wiki.vg/Protocol#Click_Window
#[derive(Transfer, Clone, Debug, PartialEq, Eq)]
pub enum ClickWindow {
  #[id = 0]
  Click(Button),
  #[id = 1]
  ShiftClick(Button),
  #[id = 2]
  Number(u8),
  #[id = 4]
  Drop,
  #[id = 5]
  DropAll,
  #[id = 6]
  DragStart(Button),
  #[id = 7]
  DragAdd(Button),
  #[id = 8]
  DragEnd(Button),
  #[id = 9]
  DoubleClick,
}

#[derive(Transfer, Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum Button {
  #[id = 0]
  #[default]
  Left,
  #[id = 1]
  Middle,
  #[id = 2]
  Right,
}


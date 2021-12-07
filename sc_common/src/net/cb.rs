use crate::{
  chunk::paletted::Section,
  math::{ChunkPos, Pos},
};

#[derive(Debug, Clone, sc_macros::Packet)]
pub enum Packet {
  BlockUpdate {
    pos:   Pos,
    state: u32,
  },
  Chat {
    msg: String,
    ty:  u8,
  },
  Chunk {
    pos:      ChunkPos,
    bit_map:  u16,
    sections: Vec<Section>,
  },
  EntityVelocity {
    eid: i32,
    x:   i16,
    y:   i16,
    z:   i16,
  },
  JoinGame {
    eid:                i32,
    hardcore_mode:      bool,
    game_mode:          u8,
    dimension:          i8,
    level_type:         String,
    difficulty:         i8,
    view_distance:      u16,
    reduced_debug_info: bool,
  },
  KeepAlive {
    id: u32,
  },
  PlayerHeader {
    header: String,
    footer: String,
  },
  SetPosLook {
    x:           f64,
    y:           f64,
    z:           f64,
    yaw:         f32,
    pitch:       f32,
    flags:       u8,
    teleport_id: u32,
  },
  UnloadChunk {
    pos: ChunkPos,
  },
  UpdateViewPos {
    pos: ChunkPos,
  },
}

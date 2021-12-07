use crate::{
  chunk::paletted::Section,
  math::{ChunkPos, Pos},
  util::{GameMode, UUID},
};

#[derive(Debug, Clone, sc_macros::Transfer)]
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
  /// Pitch/yaw change of an entity.
  EntityLook {
    eid:       i32,
    yaw:       i8,
    pitch:     i8,
    on_ground: bool,
  },
  /// Relative movement of an entity.
  EntityMove {
    eid:       i32,
    x:         i16,
    y:         i16,
    z:         i16,
    on_ground: bool,
  },
  /// Relative movement of an entity, with pitch/yaw change.
  EntityMoveLook {
    eid:       i32,
    x:         i16,
    y:         i16,
    z:         i16,
    yaw:       i8,
    pitch:     i8,
    on_ground: bool,
  },
  /// Absolute position of an entity. Also called entity teleport.
  EntityPos {
    eid:       i32,
    x:         f64,
    y:         f64,
    z:         f64,
    yaw:       i8,
    pitch:     i8,
    on_ground: bool,
  },
  /// Change of an entity's velocity.
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
  PlayerList {
    action: PlayerListAction,
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
  SpawnPlayer {
    eid:   i32,
    id:    UUID,
    x:     f64,
    y:     f64,
    z:     f64,
    yaw:   i8,
    pitch: i8,
  },
  UnloadChunk {
    pos: ChunkPos,
  },
  UpdateViewPos {
    pos: ChunkPos,
  },
}

#[derive(Debug, Clone, sc_macros::Transfer)]
pub enum PlayerListAction {
  Add(Vec<PlayerListAdd>),
  UpdateGameMode(Vec<PlayerListGameMode>),
  UpdateLatency(Vec<PlayerListLatency>),
  UpdateDisplayName(Vec<PlayerListDisplay>),
  Remove(Vec<PlayerListRemove>),
}

/// A single entry in the player list. This is what defines the tab list the
/// players see ingame. This is also how the client knows what skin to display
/// for each client. If this is not sent, the client will not spawn a player if
/// they receive a SpawnPlayer packet.
#[derive(Debug, Clone, sc_macros::Transfer)]
pub struct PlayerListAdd {
  /// Player's UUID.
  pub id:           UUID,
  /// The player's username.
  pub name:         String,
  pub game_mode:    GameMode,
  /// Their ping in milliseconds.
  pub ping:         i32,
  /// An optional display name. If present, this will replace their username in
  /// the tab list.
  pub display_name: Option<String>,
}

/// See [`PlayerListAdd`]
#[derive(Debug, Clone, sc_macros::Transfer)]
pub struct PlayerListGameMode {
  pub id:        UUID,
  pub game_mode: GameMode,
}

/// See [`PlayerListAdd`]
#[derive(Debug, Clone, sc_macros::Transfer)]
pub struct PlayerListLatency {
  pub id:   UUID,
  pub ping: i32,
}

/// See [`PlayerListAdd`]
#[derive(Debug, Clone, sc_macros::Transfer)]
pub struct PlayerListDisplay {
  pub id:           UUID,
  pub display_name: Option<String>,
}

/// See [`PlayerListAdd`]
#[derive(Debug, Clone, sc_macros::Transfer)]
pub struct PlayerListRemove {
  pub id: UUID,
}

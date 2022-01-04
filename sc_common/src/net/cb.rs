use crate::{
  chunk::paletted::Section,
  math::{ChunkPos, Pos},
  util::{GameMode, UUID},
};

#[derive(Debug, Clone, sc_macros::Transfer)]
pub enum Packet {
  Abilities {
    invulnerable: bool,
    flying:       bool,
    allow_flying: bool,
    insta_break:  bool,
    fly_speed:    f32,
    walk_speed:   f32,
  },
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
    full:     bool,
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
    game_mode:          GameMode,
    dimension:          i8,
    level_type:         String,
    difficulty:         u8,
    view_distance:      u16,
    reduced_debug_info: bool,
  },
  /// A list of changed blocks in a chunk section. This is not for a chunk
  /// column. 1.8 clients have this block for a whole chunk column, but 1.17+
  /// clients have this packet for a chunk section. It ends up being easier to
  /// just send multiple packets to 1.8 clients, as there aren't that many
  /// situations where you are changing blocks in many chunk sections at once.
  MultiBlockChange {
    /// The chunk section X and Z coordinate.
    pos:     ChunkPos,
    /// The chunk section Y coordinate.
    y:       i32,
    /// A list of relative coordinates and block ids. Each int is encoded like
    /// so: `block_id << 12 | (x << 8 | y << 4 | z)`. NOTE: This is not the same
    /// as how 1.17 encodes this! I prefer to keep x, y, z in order, as it makes
    /// more sense.
    changes: Vec<u64>,
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

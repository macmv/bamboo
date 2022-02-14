use crate::{
  chunk::{paletted::Section, BlockLight, LightChunk, SkyLight},
  math::{ChunkPos, Pos},
  util::{GameMode, UUID},
};
use std::net::SocketAddr;

#[sc_macros::transfer]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Packet {
  #[id = 0]
  Abilities {
    invulnerable: bool,
    flying:       bool,
    allow_flying: bool,
    insta_break:  bool,
    fly_speed:    f32,
    walk_speed:   f32,
  },
  #[id = 1]
  BlockUpdate { pos: Pos, state: u32 },
  #[id = 2]
  Chat { msg: String, ty: u8 },
  #[id = 3]
  Chunk {
    pos:         ChunkPos,
    full:        bool,
    bit_map:     u16,
    sections:    Vec<Section>,
    sky_light:   Option<LightChunk<SkyLight>>,
    block_light: LightChunk<BlockLight>,
  },
  #[id = 4]
  CommandList {
    nodes: Vec<CommandNode>,
    // Index into the above list
    root:  u32,
  },
  /// Pitch/yaw change of an entity.
  #[id = 5]
  EntityLook { eid: i32, yaw: i8, pitch: i8, on_ground: bool },
  /// Relative movement of an entity.
  #[id = 6]
  EntityMove { eid: i32, x: i16, y: i16, z: i16, on_ground: bool },
  /// Relative movement of an entity, with pitch/yaw change.
  #[id = 7]
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
  #[id = 8]
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
  #[id = 9]
  EntityVelocity { eid: i32, x: i16, y: i16, z: i16 },
  #[id = 10]
  JoinGame {
    eid:                   i32,
    hardcore_mode:         bool,
    game_mode:             GameMode,
    dimension:             i8,
    level_type:            String,
    difficulty:            u8,
    view_distance:         u16,
    reduced_debug_info:    bool,
    /// Only applies to 1.16+ clients.
    enable_respawn_screen: bool,
  },
  /// A list of changed blocks in a chunk section. This is not for a chunk
  /// column. 1.8 clients have this block for a whole chunk column, but 1.17+
  /// clients have this packet for a chunk section. It ends up being easier to
  /// just send multiple packets to 1.8 clients, as there aren't that many
  /// situations where you are changing blocks in many chunk sections at once.
  #[id = 11]
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
  #[id = 12]
  KeepAlive { id: u32 },
  #[id = 13]
  PlayerHeader { header: String, footer: String },
  #[id = 14]
  PlayerList { action: PlayerListAction },
  #[id = 15]
  PluginMessage { channel: String, data: Vec<u8> },
  #[id = 16]
  SetPosLook {
    x:               f64,
    y:               f64,
    z:               f64,
    yaw:             f32,
    pitch:           f32,
    flags:           u8,
    teleport_id:     u32,
    /// If set, the client will dismount any vehicle they are riding. Only
    /// applies to 1.17+ clients.
    should_dismount: bool,
  },
  #[id = 17]
  SpawnPlayer { eid: i32, id: UUID, x: f64, y: f64, z: f64, yaw: i8, pitch: i8 },
  /// A special packet. This will cause the proxy to start moving this player to
  /// a new server. If the new server accepts the connection, the proxy will
  /// simply disconnect the player from the old server. If the connection
  /// failed, then a `sb::SwitchServerFailed` packet will be sent to the server.
  #[id = 18]
  SwitchServer { ips: Vec<SocketAddr> },
  #[id = 19]
  UnloadChunk { pos: ChunkPos },
  #[id = 20]
  UpdateViewPos { pos: ChunkPos },
}

#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub struct CommandNode {
  /// The type. This is `flags & 0x03`.
  #[id = 0]
  pub ty:         CommandType,
  /// If set, then `flags & 0x04` should be set. This means the command is valid
  /// after this node. For example, `/setblock <pos> <ty>` has three nodes (lit,
  /// arg, arg). Only the last node has executable set.
  #[id = 1]
  pub executable: bool,
  /// Indices into the command nodes array
  #[id = 2]
  pub children:   Vec<u32>,
  /// If present, `flags & 0x08` must be set. Index into the command nodes
  /// array.
  #[id = 3]
  pub redirect:   Option<u32>,
  /// Only present for literal and argument nodes.
  #[id = 4]
  pub name:       String,
  /// Only present for argument nodes.
  #[id = 5]
  pub parser:     String,
  /// Only present for certain argument nodes. Format varies. This remains the
  /// same accross versions.
  #[id = 6]
  pub properties: Vec<u8>,
  /// If present, `flags & 0x10` must be set. This is a type of suggestion to
  /// give when the client is entering this node.
  #[id = 7]
  pub suggestion: Option<String>,
}

#[sc_macros::transfer]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
  #[id = 0]
  Root,
  #[id = 1]
  Literal,
  #[id = 2]
  Argument,
}

#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub enum PlayerListAction {
  #[id = 0]
  Add(Vec<PlayerListAdd>),
  #[id = 1]
  UpdateGameMode(Vec<PlayerListGameMode>),
  #[id = 2]
  UpdateLatency(Vec<PlayerListLatency>),
  #[id = 3]
  UpdateDisplayName(Vec<PlayerListDisplay>),
  #[id = 4]
  Remove(Vec<PlayerListRemove>),
}

/// A single entry in the player list. This is what defines the tab list the
/// players see ingame. This is also how the client knows what skin to display
/// for each client. If this is not sent, the client will not spawn a player if
/// they receive a SpawnPlayer packet.
#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub struct PlayerListAdd {
  /// Player's UUID.
  #[id = 0]
  pub id:           UUID,
  /// The player's username.
  #[id = 1]
  pub name:         String,
  #[id = 2]
  pub game_mode:    GameMode,
  /// Their ping in milliseconds.
  #[id = 3]
  pub ping:         i32,
  /// An optional display name. If present, this will replace their username in
  /// the tab list.
  #[id = 4]
  pub display_name: Option<String>,
}

/// See [`PlayerListAdd`]
#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub struct PlayerListGameMode {
  #[id = 0]
  pub id:        UUID,
  #[id = 1]
  pub game_mode: GameMode,
}

/// See [`PlayerListAdd`]
#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub struct PlayerListLatency {
  #[id = 0]
  pub id:   UUID,
  #[id = 1]
  pub ping: i32,
}

/// See [`PlayerListAdd`]
#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub struct PlayerListDisplay {
  #[id = 0]
  pub id:           UUID,
  #[id = 1]
  pub display_name: Option<String>,
}

/// See [`PlayerListAdd`]
#[sc_macros::transfer]
#[derive(Debug, Clone)]
pub struct PlayerListRemove {
  #[id = 0]
  pub id: UUID,
}

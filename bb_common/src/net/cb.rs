use crate::{
  chunk::{paletted::Section, BlockLight, LightChunk, SkyLight},
  math::{ChunkPos, FPos, Pos},
  metadata::Metadata,
  util::{chat::Color, Chat, GameMode, Hand, Item, UUID},
};
use bb_macros::Transfer;
use std::{collections::HashMap, net::SocketAddr};

#[derive(Transfer, Debug, Clone, PartialEq)]
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
  #[id = 32]
  Animation { eid: i32, kind: Animation },
  #[id = 1]
  BlockUpdate { pos: Pos, state: u32 },
  #[id = 38]
  ChangeGameState {
    #[must_exist]
    action: ChangeGameState,
  },
  #[id = 2]
  Chat { msg: Chat, ty: u8 },
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
  #[id = 31]
  CollectItem { item_eid: i32, player_eid: i32, amount: u8 },
  /// An entity's armor/holding item
  #[id = 33]
  EntityEquipment { eid: i32, slot: EquipmentSlot, item: Item },
  #[id = 37]
  EntityHeadLook { eid: i32, yaw: i8 },
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
  #[id = 29]
  EntityMetadata { eid: i32, ty: u32, meta: Metadata },
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
  #[id = 39]
  Particle {
    id:         i32,
    long:       bool,
    pos:        FPos,
    offset:     FPos,
    data_float: f32,
    count:      i32,
    data:       Vec<u8>,
  },
  #[id = 13]
  PlayerHeader { header: String, footer: String },
  #[id = 14]
  PlayerList {
    #[must_exist]
    action: PlayerListAction,
  },
  #[id = 36]
  PlaySound {
    name:     String,
    category: SoundCategory,
    pos:      FPos,
    volume:   f32,
    pitch:    f32,
  },
  #[id = 15]
  PluginMessage { channel: String, data: Vec<u8> },
  #[id = 30]
  RemoveEntities { eids: Vec<i32> },
  #[id = 25]
  ScoreboardDisplay {
    #[must_exist]
    position:  ScoreboardDisplay,
    objective: String,
  },
  #[id = 26]
  ScoreboardObjective {
    objective: String,
    #[must_exist]
    mode:      ObjectiveAction,
  },
  #[id = 27]
  ScoreboardUpdate {
    username:  String,
    objective: String,
    #[must_exist]
    action:    ScoreboardAction,
  },
  #[id = 16]
  SetPosLook {
    pos:             FPos,
    yaw:             f32,
    pitch:           f32,
    flags:           u8,
    teleport_id:     u32,
    /// If set, the client will dismount any vehicle they are riding. Only
    /// applies to 1.17+ clients.
    should_dismount: bool,
  },
  #[id = 21]
  SpawnLivingEntity {
    eid:      i32,
    id:       UUID,
    ty:       u32,
    pos:      FPos,
    yaw:      i8,
    pitch:    i8,
    head_yaw: i8,
    vel_x:    i16,
    vel_y:    i16,
    vel_z:    i16,
    meta:     Metadata,
  },
  #[id = 28]
  SpawnEntity {
    eid:   i32,
    id:    UUID,
    ty:    u32,
    pos:   FPos,
    yaw:   i8,
    pitch: i8,
    vel_x: i16,
    vel_y: i16,
    vel_z: i16,
    meta:  Metadata,
  },
  #[id = 17]
  SpawnPlayer {
    eid:   i32,
    id:    UUID,
    ty:    u32,
    pos:   FPos,
    yaw:   i8,
    pitch: i8,
    meta:  Metadata,
  },
  /// A special packet. This will cause the proxy to start moving this player to
  /// a new server. If the new server accepts the connection, the proxy will
  /// simply disconnect the player from the old server. If the connection
  /// failed, then a `sb::SwitchServerFailed` packet will be sent to the server.
  #[id = 18]
  SwitchServer { ips: Vec<SocketAddr> },
  #[id = 41]
  Tags { categories: HashMap<String, HashMap<String, Vec<String>>> },
  #[id = 35]
  Teams {
    team:   String,
    #[must_exist]
    action: TeamAction,
  },
  #[id = 34]
  Title {
    #[must_exist]
    action: TitleAction,
  },
  #[id = 19]
  UnloadChunk { pos: ChunkPos },
  #[id = 40]
  UpdateHealth { health: f32, food: i32, saturation: f32 },
  #[id = 20]
  UpdateViewPos { pos: ChunkPos },

  #[id = 22]
  WindowOpen { wid: u8, ty: u8, title: String },
  #[id = 23]
  WindowItems { wid: u8, items: Vec<Item>, held: Item },
  /// AKA SetSlot. I named it this so that in alphabetical order it would show
  /// up with the rest of
  /// the inventory packets.
  #[id = 24]
  WindowItem { wid: u8, slot: i32, item: Item },
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub enum ChangeGameState {
  #[id = 0]
  InvalidBed,
  #[id = 1]
  EndRaining,
  #[id = 2]
  BeginRaining,
  #[id = 3]
  GameMode(GameMode),
  #[id = 4]
  EnterCredits,
  #[id = 5]
  DemoMessage(f32),
  /// On vanilla, this is sent any time any player is hit with an arrow. Does
  /// not appear to serve a purpose.
  #[id = 6]
  ArrowHitPlayer,
  /// Used to set the darkness of the sky. Only valid between 0 and 1.
  ///
  /// This only does anything on clients 1.8-1.14.
  /// Sending this to a newer client will do nothing (the proxy will ignore it).
  #[id = 7]
  FadeValue(f32),
  /// Used to change how quickly the sky changes colors (in ticks).
  ///
  /// This only does anything on clients 1.8-1.14.
  /// Sending this to a newer client will do nothing (the proxy will ignore it).
  #[id = 8]
  FadeTime(f32),
  /// Plays a pufferfish sting sound effect.
  #[id = 9]
  PufferfishSting,
  /// Plays elder guardian effect and sound. TODO: Needs more research.
  #[id = 10]
  ElderGuardianAppear,
  /// Enables the respawn screen. If true, the respawn screen is enabled. If
  /// false, the client immediately respawns. NOTE: This only works on 1.15+
  /// clients! If you need instant respawn on older clients, you cannot let them
  /// get to 0 health.
  ///
  /// Sending this to an old client will do nothing (the proxy will ignore it).
  #[id = 11]
  EnableRespawnScreen(bool),
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub enum SoundCategory {
  #[id = 0]
  Master,
  #[id = 1]
  Music,
  #[id = 2]
  Records,
  #[id = 3]
  Weather,
  #[id = 4]
  Blocks,
  #[id = 5]
  Hostile,
  #[id = 6]
  Neutral,
  #[id = 7]
  Players,
  #[id = 8]
  Ambient,
  #[id = 9]
  Voice,
}

impl Default for SoundCategory {
  fn default() -> Self { SoundCategory::Master }
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub enum TeamAction {
  #[id = 0]
  Create {
    #[must_exist]
    info:     TeamInfo,
    entities: Vec<String>,
  },
  #[id = 1]
  Remove,
  #[id = 2]
  UpdateInfo {
    #[must_exist]
    info: TeamInfo,
  },
  #[id = 3]
  AddEntities { entities: Vec<String> },
  #[id = 4]
  RemoveEntities { entities: Vec<String> },
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub struct TeamInfo {
  pub display_name:  Chat,
  pub friendly_fire: bool,
  pub see_invis:     bool,
  #[must_exist]
  pub name_tag:      TeamRule,
  #[must_exist]
  pub collisions:    TeamRule,
  #[must_exist]
  pub color:         Color,
  pub prefix:        Chat,
  pub postfix:       Chat,
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamRule {
  #[id = 0]
  Always,
  #[id = 1]
  ForOwnTeam,
  #[id = 2]
  ForOtherTeams,
  #[id = 3]
  Never,
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub enum TitleAction {
  #[id = 0]
  Title(Chat),
  #[id = 1]
  Subtitle(Chat),
  #[id = 2]
  Times { fade_in: u32, stay: u32, fade_out: u32 },
  /// If true, the title will also be reset. If false, then sending the same
  /// times will show the same title again.
  #[id = 3]
  Clear(bool),
}

#[derive(Transfer, Debug, Clone, PartialEq)]
pub struct CommandNode {
  /// The type. This is `flags & 0x03`.
  #[id = 0]
  #[must_exist]
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

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Animation {
  #[id = 0]
  Swing(Hand),
  #[id = 1]
  Damage,
  #[id = 2]
  LeaveBed,
  #[id = 3]
  Crit,
  #[id = 4]
  MagicCrit,
}

impl Default for Animation {
  fn default() -> Self { Animation::Swing(Hand::Main) }
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
  #[id = 0]
  Hand(Hand),
  #[id = 1]
  Armor(ArmorSlot),
}

impl Default for EquipmentSlot {
  fn default() -> Self { EquipmentSlot::Hand(Hand::Main) }
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorSlot {
  #[id = 0]
  Helmet,
  #[id = 1]
  Chestplate,
  #[id = 2]
  Leggings,
  #[id = 3]
  Boots,
}

impl Default for ArmorSlot {
  fn default() -> Self { ArmorSlot::Helmet }
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
  #[id = 0]
  Root,
  #[id = 1]
  Literal,
  #[id = 2]
  Argument,
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreboardDisplay {
  #[id = 0]
  List,
  #[id = 1]
  Sidebar,
  #[id = 2]
  BelowName,
}
#[derive(Transfer, Debug, Clone, PartialEq)]
pub enum ObjectiveAction {
  #[id = 0]
  Create { value: Chat, ty: ObjectiveType },
  #[id = 1]
  Remove,
  #[id = 2]
  Update { value: Chat, ty: ObjectiveType },
}
#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveType {
  #[id = 0]
  Integer,
  #[id = 1]
  Hearts,
}
#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreboardAction {
  #[id = 0]
  Create(i32),
  #[id = 1]
  Remove,
}

impl Default for ObjectiveType {
  fn default() -> Self { ObjectiveType::Integer }
}

#[derive(Transfer, Debug, Clone, PartialEq)]
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
#[derive(Transfer, Debug, Default, Clone, PartialEq)]
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
#[derive(Transfer, Debug, Default, Clone, PartialEq)]
pub struct PlayerListGameMode {
  pub id:        UUID,
  pub game_mode: GameMode,
}

/// See [`PlayerListAdd`]
#[derive(Transfer, Debug, Default, Clone, PartialEq)]
pub struct PlayerListLatency {
  pub id:   UUID,
  pub ping: i32,
}

/// See [`PlayerListAdd`]
#[derive(Transfer, Debug, Default, Clone, PartialEq)]
pub struct PlayerListDisplay {
  pub id:           UUID,
  pub display_name: Option<Chat>,
}

/// See [`PlayerListAdd`]
#[derive(Transfer, Debug, Default, Clone, PartialEq)]
pub struct PlayerListRemove {
  pub id: UUID,
}

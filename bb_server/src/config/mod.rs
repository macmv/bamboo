use bb_common::{math::FPos, util::GameMode};
use bb_macros::{Config, Default};
use log::LevelFilter;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct Config {
  /// Only show info logs by default.
  #[default(LevelFilter::Info)]
  pub log_level: LevelFilter,

  /// Don't log any packets. This is just for debugging. If you need to debug
  /// packets, you can add "all", which will log every packet. You can also
  /// add a specific packet name (such as "Flying"), which will only log
  /// packets with that name.
  pub log_packets: Vec<String>,

  /// The address the server will listen for connections on. Vanilla clients
  /// cannot connect to this address! The proxy must be configured to connect
  /// to this address instead.
  #[default("0.0.0.0:8483".into())]
  pub address: String,

  /// The default view distance. Note that this can be changed for a single
  /// player via a plugin at runtime.
  #[default(10)]
  pub view_distance: u32,

  /// Whenever a player joins, they will be put into this gamemode. This can
  /// be overriden with plugins, but without any plugins, this will be the
  /// gamemode of all the clients.
  ///
  /// Can be one of:
  /// - creative
  /// - survival
  /// - adventure
  /// - spectator
  #[default(GameMode::Creative)]
  pub default_gamemode: GameMode,

  /// The place where everyone spawns in within the world.
  #[default(FPos::new(0.0, 64.0, 0.0))]
  pub spawn_point: FPos,

  /// If true, the world will search upwards from the given spawn point for a
  /// suitable location every time a player is spawned. If false, then the world
  /// will simply place new players at the spawn point (even if they suffocate).
  #[default(true)]
  pub find_spawn: bool,

  /// If true, when players join, a chat message will be displayed.
  #[default(true)]
  pub join_messages:  bool,
  /// If true, when a player leaves, a chat message will be displayed.
  #[default(true)]
  pub leave_messages: bool,

  /// The path for the vanilla data directory. If not found, an error will be
  /// logged, and there will be no crafting recipes.
  #[default("data/".into())]
  pub data_path: String,

  /// Toggle debug info in the player list.
  #[default(true)]
  pub debug_playerlist: bool,

  /// Configs for rcon. This is a protocol used by vanilla to allow a remote
  /// to execute commands on the server.
  pub rcon: RconConfig,

  /// Configs for world generation/loading.
  pub world: WorldConfig,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct RconConfig {
  /// By default this is disabled. This is for security. Anyone with access
  /// to the port will be able to execute commands on this server.
  #[default(false)]
  pub enabled:  bool,
  /// This is the port that rcon connects listens on.
  #[default("0.0.0.0:25575".into())]
  pub addr:     String,
  /// This is the password to use when connecting. This is basically
  /// meaningless, as the connection is entirely unencrypted. So, if you have
  /// the above address open to anyone, consider it completely vulnerable to
  /// attacks.
  ///
  /// Note that the password is always required.
  pub password: String,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct WorldConfig {
  /// If set, the world cannot be modified. This can be used in minigame
  /// lobbies, for example.
  #[default(false)]
  pub locked: bool,
  /// If set, the world will be saved to disk.
  #[default(true)]
  pub save:   bool,

  /// Generation settings

  /// If set, then the entire world will be filled with debug blocks.
  #[default(false)]
  pub debug:     bool,
  /// If set, the whole world will be void.
  #[default(false)]
  pub void:      bool,
  /// This can be set to change the world's terrain generator. Generators are
  /// added by plugins. If the generator is not present, the server will fail
  /// to load.
  #[default("".into())]
  pub generator: String,

  /// The height of this world. This is 1 block larger than the maximum block.
  #[default(256)]
  pub height: u32,
  /// The minimum Y value of this world. This is the lowest block you can place.
  #[default(0)]
  pub min_y:  i32,

  /// Vanilla world loading settings
  pub vanilla: VanillaConfig,

  /// Schematic reading settings.
  pub schematic: SchematicConfig,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct VanillaConfig {
  /// If set, then the world will be a void world, and a vanilla world will
  /// be loaded when the server starts. This loads first, so schematics will
  /// replace any blocks set by the vanilla world.
  #[default(false)]
  pub enabled: bool,
  /// The path to the world. This should be a path to a world containing a
  /// `chunks` folder, which should contain all of the chunks of the world.
  /// The value of this path is ignored if vanilla loading is not enabled.
  #[default("".into())]
  pub path:    String,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct SchematicConfig {
  /// If set, then the world will be a void world, and a schematic will be
  /// loaded from the given path on server startup. This overrides any of
  /// the options below.
  #[default(false)]
  pub enabled: bool,
  /// The path to the schematic file. The value of this path is ignored
  /// if schematic loading is not enabled.
  #[default("".into())]
  pub path:    String,
}

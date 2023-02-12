use bb_macros::{Config, Default};
use log::LevelFilter;

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct Config {
  /// Only show info logs by default.
  #[default(LevelFilter::Info)]
  pub log_level: LevelFilter,

  /// The Bamboo server's IP.
  #[default("0.0.0.0:8483".into())]
  pub server:  String,
  /// The IP of the proxy. This is the IP that all clients will connect to.
  #[default("0.0.0.0:25565".into())]
  pub address: String,

  /// This enables authentication with Mojang's servers. This should only be
  /// disabled if you know what you are doing.
  #[default(true)]
  pub encryption: bool,
  /// This is for receiving player data from another proxy such as Velocity.
  #[default(Forwarding::None)]
  pub forwarding: Forwarding,

  #[default("A Bamboo Server".into())]
  pub motd: String,

  #[default(20)]
  pub max_players:        i32,
  /// This is the packet compression threshold. Vanilla clients will perform
  /// far worse if this is turned off. Compression can be disabled by setting
  /// this to -1. The proxy will compress all packets if this is set to 0.
  #[default(256)]
  pub compression_thresh: i32,
  /// The path to the icon.
  #[default("icon.png".into())]
  pub icon:               String,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct BedrockConfig {
  pub address: String,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub enum Forwarding {
  /// No forwarding will be used. This is the default.
  #[default]
  None,
  /// The proxy will parse player info from the client using BungeeCord format.
  /// This will allow any incoming connection to login with arbitrary profiles.
  /// This should only be used if you know what you are doing.
  Legacy,
}

use bb_macros::{Config, Default};

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct Config {
  /// The type of plugin. For socket based plugins, use 'socket', and
  /// for panda plugins, use 'panda'
  #[default("panda".into())]
  pub plugin_type: String,

  /// If set to true, then this plugin will be loaded. If set to false,
  /// this plugin will be ignored.
  #[default(true)]
  pub enabled: bool,

  /// Socket-specific configs
  pub socket: SocketConfig,

  /// Wasm specific settings.
  pub wasm: WasmConfig,

  /// Panda-specific configs
  ///
  /// Nothing here yet
  pub panda: PandaConfig,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct SocketConfig {
  /// This is the command the server should run to start the plugin.
  /// All output will be captured from this program, and included in
  /// log messages. This command is relative to the plugin root.
  pub entrypoint: String,
}

#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct WasmConfig {
  /// The command to run to compile the wasm. If empty, no command
  /// will be run.
  pub compile: String,
  /// The path to the compiled wasm.
  pub output:  String,
}
#[derive(Clone, Debug, Config, Default, PartialEq)]
pub struct PandaConfig {}

use super::{panda::PandaPlugin, JsonPlayer, JsonPos};
use sc_common::config::Config;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "t")]
pub enum PluginEvent {
  Register { ty: String },
  Ready,

  SendChat { text: String },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServerEvent {
  pub player: JsonPlayer,
  #[serde(flatten)]
  pub kind:   ServerEventKind,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "t")]
pub enum ServerEventKind {
  BlockPlace { pos: JsonPos },
  Chat { text: String },
}

pub trait PluginImpl: std::any::Any {
  /// If this returns an error, the plugin will be removed, and this function
  /// will not be called again.
  fn call(&self, event: ServerEvent) -> Result<(), ()>;
  fn panda(&mut self) -> Option<&mut PandaPlugin> { None }
}

pub struct Plugin {
  config: Config,
  name:   String,
  imp:    Box<dyn PluginImpl + Send + Sync>,
}

impl Plugin {
  pub fn new(config: Config, name: String, imp: impl PluginImpl + Send + Sync + 'static) -> Self {
    Plugin { config, name, imp: Box::new(imp) }
  }
  pub fn call(&self, ev: ServerEvent) -> Result<(), ()> { self.imp.call(ev) }
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { self.imp.panda().unwrap() }
}

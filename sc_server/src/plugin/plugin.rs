use super::{panda::PandaPlugin, JsonBlock, JsonPlayer, JsonPos};
use sc_common::config::Config;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum PluginMessage {
  Event {
    #[serde(flatten)]
    event: PluginEvent,
  },
  Request {
    reply_id: u32,
    #[serde(flatten)]
    request:  PluginRequest,
  },
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginEvent {
  Register { ty: String },
  Ready,

  SendChat { text: String },
}
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginRequest {
  GetBlock { pos: JsonPos },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub enum ServerMessage {
  Event {
    player: JsonPlayer,
    #[serde(flatten)]
    event:  ServerEvent,
  },
  Reply {
    reply_id: u32,
    #[serde(flatten)]
    reply:    ServerReply,
  },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
  BlockPlace { pos: JsonPos },
  Chat { text: String },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerReply {
  Block { pos: JsonPos, block: JsonBlock },
}

pub trait PluginImpl: std::any::Any {
  /// If this returns an error, the plugin will be removed, and this function
  /// will not be called again.
  fn call(&self, event: ServerMessage) -> Result<(), ()>;
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
  pub fn call(&self, ev: ServerMessage) -> Result<(), ()> { self.imp.call(ev) }
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { self.imp.panda().unwrap() }
}

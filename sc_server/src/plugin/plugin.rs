use super::panda::PandaPlugin;
use sc_common::config::Config;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginEvent {
  Register { ty: String },
  Ready,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
  BlockPlace { pos: (i32, i32, i32) },
}

pub trait PluginImpl: std::any::Any {
  fn call(&self, event: ServerEvent);
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
  pub fn call(&self, ev: ServerEvent) { self.imp.call(ev); }
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { self.imp.panda().unwrap() }
}

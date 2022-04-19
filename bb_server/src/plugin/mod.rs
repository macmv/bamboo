#[cfg(feature = "panda_plugins")]
pub mod panda;
#[cfg(feature = "python_plugins")]
pub mod python;
#[cfg(feature = "socket_plugins")]
pub mod socket;
#[cfg(feature = "wasm_plugins")]
pub mod wasm;

#[cfg(not(doctest))]
mod types;

pub mod json;
mod manager;
mod message;

pub use manager::PluginManager;
pub use message::{
  GlobalServerEvent, PluginEvent, PluginMessage, PluginReply, PluginRequest, ServerEvent,
  ServerMessage, ServerReply, ServerRequest,
};

#[cfg(feature = "socket_plugins")]
use socket::SocketManager;

use crate::{block, player::Player, world::WorldManager};
use ::panda::runtime::VarSend;
use bb_common::{config::Config, math::Pos};
use parking_lot::Mutex;
use std::{error::Error, fmt, sync::Arc};

#[derive(Debug)]
pub enum Event {
  Init,
  OnBlockPlace(Arc<Player>, Pos, block::Kind),
}

#[derive(Clone)]
#[cfg_attr(feature = "python_plugins", ::pyo3::pyclass)]
pub struct Bamboo {
  // Index into plugins array
  idx:  usize,
  wm:   Arc<WorldManager>,
  // Locking this removes the value. If the value is none, then this enters a wait loop until there
  // is a value present.
  //
  // This is not by any means "fast", but it will work as long as a thread doesn't lock this for
  // too long.
  data: Arc<Mutex<Option<VarSend>>>,
}

impl Bamboo {
  pub fn new(idx: usize, wm: Arc<WorldManager>) -> Self {
    Bamboo { idx, wm, data: Arc::new(Mutex::new(Some(VarSend::None))) }
  }
}

impl fmt::Debug for Bamboo {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Bamboo {{}}") }
}

#[cfg(feature = "panda_plugins")]
use self::panda::PandaPlugin;

pub trait PluginImpl: std::any::Any {
  /// If this returns an error, the plugin will be removed, and this function
  /// will not be called again.
  ///
  /// If this returns `false`, the event will be cancelled.
  fn call(&self, event: ServerMessage) -> Result<bool, CallError>;
  #[cfg(feature = "panda_plugins")]
  fn panda(&mut self) -> Option<&mut PandaPlugin> { None }
}

pub struct Plugin {
  #[allow(unused)]
  config: Config,
  imp:    Box<dyn PluginImpl + Send + Sync>,
}

#[derive(Debug)]
pub struct CallError {
  pub keep:  bool,
  pub inner: Box<dyn Error>,
}

impl CallError {
  pub fn no_keep(inner: impl Error + 'static) -> Self {
    CallError { keep: false, inner: Box::new(inner) }
  }
  pub fn keep(inner: impl Error + 'static) -> Self {
    CallError { keep: true, inner: Box::new(inner) }
  }
}

impl fmt::Display for CallError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.inner)?;
    if self.keep {
      write!(f, " (plugin still valid)")
    } else {
      write!(f, " (plugin no longer valid)")
    }
  }
}

impl Error for CallError {}

impl Plugin {
  pub fn new(config: Config, imp: impl PluginImpl + Send + Sync + 'static) -> Self {
    Plugin { config, imp: Box::new(imp) }
  }
  pub fn call(&self, ev: ServerMessage) -> Result<bool, CallError> { self.imp.call(ev) }
  #[cfg(feature = "panda_plugins")]
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { self.imp.panda().unwrap() }
}

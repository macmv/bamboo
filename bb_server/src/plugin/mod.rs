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
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use parking_lot::Mutex;
use std::{error::Error, fmt, sync::Arc, thread, time::Duration};

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
  /// Calls an event. There is no reply for `ServerEvent`. If an error is
  /// thrown, it will be logged, and the plugin will be removed if `keep` is
  /// `false`.
  fn call(&self, player: Arc<Player>, event: ServerEvent) -> Result<(), CallError>;
  fn call_global(&self, event: GlobalServerEvent) -> Result<(), CallError>;
  /// Calls an event. This should block until it gets a reply.
  fn req(&self, player: Arc<Player>, event: ServerRequest) -> Result<PluginReply, CallError>;
  #[cfg(feature = "panda_plugins")]
  fn panda(&mut self) -> Option<&mut PandaPlugin> { None }
}

pub struct Plugin {
  // This will be useful in the future. Probably.
  #[allow(unused)]
  config:   Config,
  name:     String,
  // TODO: Maybe use a mutex or something, so that we can get `unwrap_panda` to work again.
  #[allow(unused)]
  imp:      Arc<dyn PluginImpl + Send + Sync>,
  reply_id: u32,
  tx:       Sender<ServerMessage>,
  rx:       Receiver<PluginMessage>,
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

impl CallError {
  pub fn log(&self) {
    error!("{self}");
  }
}

impl Plugin {
  pub fn new(name: String, config: Config, imp: impl PluginImpl + Send + Sync + 'static) -> Self {
    let (server_tx, server_rx) = crossbeam_channel::bounded(128);
    let (plugin_tx, plugin_rx) = crossbeam_channel::bounded(128);
    let imp = Arc::new(imp);
    let i = Arc::clone(&imp);
    thread::spawn(move || {
      while let Ok(ev) = server_rx.recv() {
        let res = match ev {
          ServerMessage::Request { reply_id, player, request } => i
            .req(player, request)
            .map(|reply| plugin_tx.send(PluginMessage::Reply { reply_id, reply }).unwrap()),
          ServerMessage::Event { player, event } => i.call(player, event),
          ServerMessage::GlobalEvent { event } => i.call_global(event),
          ServerMessage::Reply { .. } => Ok(()),
        };
        match res {
          Ok(()) => (),
          Err(e) => {
            e.log();
            if !e.keep {
              return;
            }
          }
        }
      }
    });
    Plugin { config, name, imp, reply_id: 0, tx: server_tx, rx: plugin_rx }
  }
  pub fn call(&self, ev: ServerMessage, timeout: Duration) -> Result<bool, CallError> {
    let reply = match ev {
      ServerMessage::Request { reply_id, .. } => Some(reply_id),
      _ => None,
    };
    self.tx.send(ev).unwrap();
    if let Some(_reply) = reply {
      match self.rx.recv_timeout(timeout) {
        Ok(res) => match res {
          PluginMessage::Reply { reply_id: _, reply } => match reply {
            PluginReply::Cancel { allow } => Ok(allow),
          },
          _ => todo!(),
        },
        Err(RecvTimeoutError::Timeout) => {
          warn!("event timed out for plugin `{}`", self.name);
          Ok(true)
        }
        Err(e @ RecvTimeoutError::Disconnected) => {
          error!("plugin {} disconnected", self.name);
          Err(CallError::no_keep(e))
        }
      }
    } else {
      Ok(true)
    }
  }
  pub fn next_reply(&mut self) -> u32 {
    self.reply_id += 1;
    self.reply_id
  }
  #[cfg(feature = "panda_plugins")]
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { todo!() }
  /*
  #[cfg(feature = "panda_plugins")]
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { self.imp.panda().unwrap() }
  */
}

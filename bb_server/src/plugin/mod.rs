#[cfg(feature = "panda_plugins")]
pub mod panda;
#[cfg(feature = "python_plugins")]
pub mod python;
#[cfg(feature = "socket_plugins")]
pub mod socket;
#[cfg(feature = "wasm_plugins")]
pub mod wasm;

#[cfg(not(doctest))]
pub mod types;

#[cfg(doctest)]
pub mod types {
  use std::fmt;

  pub mod event {
    pub struct EventFlow;
  }

  pub trait Callback: fmt::Debug + Send + Sync {
    #[cfg(feature = "panda_plugins")]
    fn call_panda(
      &self,
      env: &mut panda::runtime::LockedEnv<'_>,
      args: Vec<panda::runtime::Var>,
    ) -> panda::runtime::Result<()> {
      let _ = (env, args);
      panic!("cannot call this callback in panda");
    }
    #[cfg(feature = "python_plugins")]
    fn call_python(&self, args: Vec<pyo3::PyObject>) {
      panic!("cannot call this callback in python");
    }

    fn box_clone(&self) -> Box<dyn Callback>;
  }
}

pub mod config;
mod manager;

pub use self::panda::IntoPanda;
pub use manager::PluginManager;

#[cfg(feature = "socket_plugins")]
use socket::SocketManager;

use crate::{
  event::{GlobalEvent, PlayerEvent, PlayerRequest, PluginMessage, PluginReply, ServerMessage},
  world::WorldManager,
};
use ::panda::runtime::{tree::Closure, LockedEnv, VarSend};
use config::Config;
use crossbeam_channel::{Receiver, Sender};
use parking_lot::{Mutex, MutexGuard};
use std::{error::Error, fmt, sync::Arc, thread};

#[derive(Debug)]
struct Scheduled {
  runnable:     Runnable,
  repeat:       bool,
  initial_time: u32,
  time_left:    u32,
}
enum Runnable {
  Fn(Box<dyn Fn(&mut LockedEnv) + Send + Sync>),
  Closure(Closure),
}

impl fmt::Debug for Runnable {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Fn(_) => write!(f, "Fn(..)"),
      Self::Closure(_) => write!(f, "Closure(..)"),
    }
  }
}

impl Scheduled {
  pub fn new(ticks: u32, runnable: Runnable) -> Self {
    Scheduled { runnable, repeat: false, time_left: ticks, initial_time: ticks }
  }
  pub fn new_loop(ticks: u32, runnable: Runnable) -> Self {
    Scheduled { runnable, repeat: true, time_left: ticks, initial_time: ticks }
  }
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

  scheduled:        Arc<Mutex<Vec<Scheduled>>>,
  /// Used as a backup if scheduled is locked. This is so that we can schedule
  /// things within a scheduled callback. This will never be locked while
  /// `scheduled` is locked.
  scheduled_backup: Arc<Mutex<Vec<Scheduled>>>,
}

impl Bamboo {
  pub fn new(idx: usize, wm: Arc<WorldManager>) -> Self {
    Bamboo {
      idx,
      wm,
      data: Arc::new(Mutex::new(Some(VarSend::None))),
      scheduled: Arc::new(Mutex::new(vec![])),
      scheduled_backup: Arc::new(Mutex::new(vec![])),
    }
  }
}

impl fmt::Debug for Bamboo {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Bamboo {{}}") }
}

#[cfg(feature = "panda_plugins")]
use self::panda::PandaPlugin;

pub trait PluginImpl: std::any::Any {
  /// Calls an event. There is no reply for `GlobalEvent`.
  fn call_global(&self, event: GlobalEvent) -> Result<(), CallError>;
  /// Calls an event. There is no reply for `PlayerEvent`. If an error is
  /// thrown, it will be logged, and the plugin will be removed if `keep` is
  /// `false`.
  fn call(&self, event: PlayerEvent) -> Result<(), CallError>;
  /// Calls an event. This should block until it gets a reply.
  fn req(&self, event: PlayerRequest) -> Result<PluginReply, CallError>;
  #[cfg(feature = "panda_plugins")]
  fn panda(&mut self) -> Option<&mut PandaPlugin> { None }
}

pub struct Plugin {
  // This will be useful in the future. Probably.
  #[allow(unused)]
  config:    Config,
  #[allow(unused)]
  name:      String,
  imp:       Arc<Mutex<dyn PluginImpl + Send + Sync>>,
  tx:        Sender<ServerMessage>,
  rx:        Receiver<PluginMessage>,
  /// Used to recycled events we don't care about back into the queue.
  plugin_tx: Sender<PluginMessage>,
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
    let imp = Arc::new(Mutex::new(imp));
    let i = Arc::clone(&imp);
    let ptx = plugin_tx.clone();
    thread::spawn(move || {
      while let Ok(ev) = server_rx.recv() {
        let res = match ev {
          ServerMessage::PlayerRequest { reply_id, request } => i
            .lock()
            .req(request)
            .map(|reply| plugin_tx.send(PluginMessage::Reply { reply_id, reply }).unwrap()),
          ServerMessage::PlayerEvent { event } => i.lock().call(event),
          ServerMessage::GlobalEvent { event } => i.lock().call_global(event),
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
    Plugin { config, name, imp, tx: server_tx, rx: plugin_rx, plugin_tx: ptx }
  }
  pub fn tick(&self) {
    if let Some(pd) = self.imp.lock().panda() {
      let bb = pd.bb();
      let mut lock = bb.scheduled.lock();
      lock.retain_mut(|s| {
        s.time_left -= 1;
        if s.time_left == 0 {
          match &s.runnable {
            Runnable::Closure(c) => match c.call(&mut pd.lock_env(), vec![]) {
              Ok(_) => {}
              Err(e) => pd.print_err(e),
            },
            Runnable::Fn(f) => f(&mut pd.lock_env()),
          }
          s.time_left = s.initial_time;
          s.repeat
        } else {
          true
        }
      });
      // During the above calls, more tasks may have been scheduled, in which case
      // they will have been added to scheduled_backup. Here we drain those elements
      // into the normal `scheduled` list.
      lock.extend(bb.scheduled_backup.lock().drain(..));
    }
  }
  pub fn call_global(&self, event: GlobalEvent) -> Result<(), CallError> {
    self.tx.send(ServerMessage::GlobalEvent { event }).unwrap();
    Ok(())
  }
  pub fn call(&self, event: PlayerEvent) -> Result<(), CallError> {
    self.tx.send(ServerMessage::PlayerEvent { event }).unwrap();
    Ok(())
  }
  pub fn req(&self, reply_id: u32, request: PlayerRequest) -> Result<(), CallError> {
    self.tx.send(ServerMessage::PlayerRequest { reply_id, request }).unwrap();
    Ok(())
  }
  pub fn rx(&self) -> &Receiver<PluginMessage> { &self.rx }
  /// `Some(true)` means we allow.
  /// `Some(false)` means we disallow.
  /// `None` means this is a message we don't care about.
  pub(crate) fn check_allow(&self, msg: PluginMessage, now: u32, rid: u32) -> Option<bool> {
    match &msg {
      PluginMessage::Reply { reply_id, reply } => {
        // If it is too old, we discard this message. The listener for this reply has
        // probably already exited, so we just ignore it.
        if reply_id + 50_000 < now {
          return None;
        }
        if *reply_id == rid {
          match reply {
            PluginReply::Cancel { allow } => return Some(*allow),
          }
        }
      }
      _ => self.plugin_tx.send(msg).unwrap(),
    }
    None
  }
  pub fn lock_imp(&self) -> MutexGuard<'_, dyn PluginImpl + Send + Sync> { self.imp.lock() }
  /*
  #[cfg(feature = "panda_plugins")]
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { self.imp.panda().unwrap() }
  */
}

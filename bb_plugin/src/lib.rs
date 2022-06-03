pub use bb_ffi as ffi;
pub use log::*;

use bb_common::util::Chat;
use bb_ffi::CChat;
use parking_lot::Mutex;
use std::marker::PhantomData;

pub use bb_common::{chunk, math, transfer, util};

pub mod block;
pub mod command;
pub mod entity;
mod ffi_impls;
pub mod item;
pub mod particle;
pub mod player;
pub mod time;
pub mod world;

mod internal;

pub use command::add_command;
pub use internal::gen::add_world_generator;

pub struct Bamboo {
  marker: PhantomData<()>,
}

pub fn instance() -> Bamboo { Bamboo { marker: PhantomData::default() } }

pub trait IntoFfi {
  type Ffi;

  fn into_ffi(self) -> Self::Ffi;
}

use bb_common::util::UUID;
use parking_lot::MutexGuard;
use std::{any::Any, collections::HashMap};

pub trait PlayerStore: Any + Send {
  fn as_any(&mut self) -> &mut dyn Any;
  fn new() -> Self
  where
    Self: Sized;
}

pub struct PluginStore {
  players: Option<HashMap<UUID, Box<dyn PlayerStore>>>,
}

impl PluginStore {
  const fn new() -> PluginStore { PluginStore { players: None } }
  pub fn player<T: PlayerStore>(&mut self, id: UUID) -> &mut T {
    let b = match &mut self.players {
      Some(p) => p.entry(id).or_insert_with(|| Box::new(T::new())),
      p @ None => {
        *p = Some(HashMap::new());
        p.as_mut().unwrap().entry(id).or_insert_with(|| Box::new(T::new()))
      }
    };
    b.as_any().downcast_mut().expect("wrong type given for player store")
  }
}

static STORE: Mutex<PluginStore> =
  Mutex::const_new(parking_lot::RawMutex::INIT, PluginStore::new());

impl Bamboo {
  pub fn broadcast(&self, message: Chat) {
    unsafe {
      let c_chat = CChat { message: bb_ffi::CStr::new(message.to_codes()) };
      bb_ffi::bb_broadcast(&c_chat);
    }
  }
  pub fn store(&self) -> MutexGuard<PluginStore> { STORE.lock() }
}

use log::{Level, LevelFilter, Metadata, Record};

struct Logger;
static LOGGER: Logger = Logger;

impl log::Log for Logger {
  fn enabled(&self, metadata: &Metadata) -> bool { metadata.level() <= Level::Info }

  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {
      unsafe {
        if let Some(s) = record.args().as_str() {
          bb_ffi::bb_log(
            record.level() as u32,
            s.as_ptr(),
            s.len() as u32,
            record.target().as_ptr(),
            record.target().len() as u32,
            record.module_path().unwrap_or("").as_ptr(),
            record.module_path().unwrap_or("").len() as u32,
            record.file().unwrap_or("").as_ptr(),
            record.file().unwrap_or("").len() as u32,
            record.line().unwrap_or(0),
          );
        } else {
          let s = record.args().to_string();
          bb_ffi::bb_log(
            record.level() as u32,
            s.as_ptr(),
            s.len() as u32,
            record.target().as_ptr(),
            record.target().len() as u32,
            record.module_path().unwrap_or("").as_ptr(),
            record.module_path().unwrap_or("").len() as u32,
            record.file().unwrap_or("").as_ptr(),
            record.file().unwrap_or("").len() as u32,
            record.line().unwrap_or(0),
          );
        }
      }
    }
  }
  fn flush(&self) {}
}

pub fn init() {
  std::panic::set_hook(Box::new(|info| {
    match (info.payload().downcast_ref::<&str>(), info.location()) {
      (Some(s), Some(location)) => {
        error!("plugin panic: {s:?} at {}:{}", location.file(), location.line())
      }
      (Some(s), None) => error!("plugin panic: {s:?} at <no location>"),
      (None, Some(location)) => {
        error!("plugin panic: <no message> at {}:{}", location.file(), location.line())
      }
      (None, None) => error!("plugin panic: <no message> at <no location>"),
    }
  }));
  log::set_logger(&LOGGER).unwrap();
  log::set_max_level(LevelFilter::Info);
}

use parking_lot::lock_api::RawMutex;

macro_rules! callback {
  ( $setter:ident, $static:ident, $sig:ty ) => {
    static $static: Mutex<Option<Box<dyn ($sig) + Send>>> =
      Mutex::const_new(parking_lot::RawMutex::INIT, None);
    pub fn $setter(callback: impl ($sig) + Send + 'static) {
      *$static.lock() = Some(Box::new(callback));
    }
  };
}

callback!(set_on_block_place, ON_BLOCK_PLACE, Fn(player::Player, math::Pos) -> bool);
#[no_mangle]
extern "C" fn on_block_place(id: ffi::CUUID, x: i32, y: i32, z: i32) -> bool {
  if let Some(cb) = ON_BLOCK_PLACE.lock().as_ref() {
    let p = player::Player::new(id);
    let pos = math::Pos { x, y, z };
    cb(p, pos)
  } else {
    true
  }
}

callback!(set_on_tick, ON_TICK, Fn());
#[no_mangle]
extern "C" fn on_tick() {
  // If we fail to lock, we just don't process this update. It kinda sucks, but
  // parking_lot will panic if we block.
  if let Some(lock) = ON_TICK.try_lock() {
    if let Some(cb) = lock.as_ref() {
      cb()
    }
  }
}

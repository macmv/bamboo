pub use bb_ffi as ffi;
pub use log::*;

use bb_common::util::Chat;
use bb_ffi::CChat;
use std::marker::PhantomData;

pub use bb_common::{chunk, transfer, util};

mod ffi_impls;
mod internal;
mod store;

pub mod block;
pub mod command;
pub mod entity;
pub mod item;
pub mod math;
pub mod particle;
pub mod player;
pub mod sync;
pub mod time;
pub mod world;

pub use command::add_command;
pub use internal::gen::add_world_generator;
pub use store::{store, PlayerStore, PluginStore};

pub struct Bamboo {
  marker: PhantomData<()>,
}

pub fn instance() -> Bamboo { Bamboo { marker: PhantomData::default() } }

pub trait IntoFfi {
  type Ffi;

  fn into_ffi(self) -> Self::Ffi;
}

pub trait FromFfi {
  type Ffi;

  fn from_ffi(f: Self::Ffi) -> Self;
}

use sync::ConstLock;

impl Bamboo {
  pub fn broadcast(&self, message: Chat) {
    unsafe {
      let c_chat = CChat { message: bb_ffi::CStr::new(message.to_codes()) };
      bb_ffi::bb_broadcast(&c_chat);
    }
  }
}

use log::{Level, LevelFilter, Metadata, Record};

struct Logger;
static LOGGER: Logger = Logger;

impl log::Log for Logger {
  fn enabled(&self, metadata: &Metadata) -> bool { metadata.level() <= Level::Debug }

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
    let msg = if let Some(msg) = info.payload().downcast_ref::<&str>() {
      *msg
    } else if let Some(msg) = info.payload().downcast_ref::<String>() {
      msg.as_str()
    } else {
      "<no message>"
    };
    if let Some(loc) = info.location() {
      error!("plugin panic: {msg:?} at {}:{}", loc.file(), loc.line());
    } else {
      error!("plugin panic: {msg:?} at <no location>");
    }
  }));
  log::set_logger(&LOGGER).unwrap();
  log::set_max_level(LevelFilter::Debug);
}

macro_rules! callback {
  ( $setter:ident, $static:ident, $($sig:tt)* ) => {
    static $static: ConstLock<Option<Box<dyn ($($sig)*) + Send>>> = ConstLock::new(None);
    pub fn $setter(callback: impl ($($sig)*) + Send + 'static) {
      *$static.lock() = Some(Box::new(callback));
    }
  };
}

callback!(set_on_block_place, ON_BLOCK_PLACE, Fn(player::Player, math::Pos) -> bool);
#[no_mangle]
extern "C" fn on_block_place(id: ffi::CUUID, x: i32, y: i32, z: i32) -> bool {
  if let Some(cb) = ON_BLOCK_PLACE.lock().as_ref() {
    let p = player::Player::from_ffi(id);
    let pos = math::Pos { x, y, z };
    cb(p, pos)
  } else {
    true
  }
}

callback!(set_on_tick, ON_TICK, Fn());
#[no_mangle]
extern "C" fn on_tick() {
  // If we fail to lock, we just don't process this update. This is intentional,
  // as it means the old tick handler is still running, which is a Bad Thing.
  if let Some(lock) = ON_TICK.try_lock() {
    if let Some(cb) = lock.as_ref() {
      cb()
    }
  }
}

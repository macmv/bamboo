pub use bb_ffi as ffi;
pub use log::*;

use bb_ffi::CChat;
use std::{ffi::CString, marker::PhantomData};

use bb_common::util::Chat;

pub use bb_common::{math, util};

pub mod player;

pub struct Bamboo {
  marker: PhantomData<()>,
}

pub fn instance() -> Bamboo { Bamboo { marker: PhantomData::default() } }

impl Bamboo {
  pub fn broadcast(&self, message: Chat) {
    unsafe {
      let s = CString::new(message.to_codes()).unwrap();
      let c_chat = CChat { message: s.as_ptr() };
      bb_ffi::bb_broadcast(&c_chat);
    }
  }
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
          bb_ffi::bb_log_len(record.level() as u32, s.as_ptr() as *const _, s.len() as u32);
        } else {
          let s = record.args().to_string();
          bb_ffi::bb_log_len(record.level() as u32, s.as_ptr() as *const _, s.len() as u32);
        }
      }
    }
  }
  fn flush(&self) {}
}

pub fn init() {
  log::set_logger(&LOGGER).unwrap();
  log::set_max_level(LevelFilter::Info);
}

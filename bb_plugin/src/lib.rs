pub use bb_ffi as ffi;
pub use log::*;

use bb_ffi::CChat;
use parking_lot::Mutex;
use std::{collections::HashMap, ffi::CString, marker::PhantomData};

use bb_common::{
  chunk::{paletted, Chunk},
  math::{ChunkPos, RelPos},
  transfer::MessageWriter,
  util::Chat,
};

pub use bb_common::{chunk, math, util};

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

callback!(set_on_block_place, ON_BLOCK_PLACE, Fn(player::Player, math::Pos));
#[no_mangle]
extern "C" fn on_block_place(id: ffi::CUUID, x: i32, y: i32, z: i32) {
  if let Some(cb) = ON_BLOCK_PLACE.lock().as_ref() {
    let p = player::Player::new(id);
    let pos = math::Pos { x, y, z };
    cb(p, pos);
  }
}

static CHUNK_BUF: Mutex<Option<Vec<u8>>> = Mutex::const_new(parking_lot::RawMutex::INIT, None);
static GENERATORS: Mutex<
  Option<HashMap<String, Box<dyn Fn(&mut Chunk<paletted::Section>, ChunkPos) + Send>>>,
> = Mutex::const_new(parking_lot::RawMutex::INIT, None);

pub fn add_world_generator(
  name: &str,
  func: impl Fn(&mut Chunk<paletted::Section>, ChunkPos) + Send + 'static,
) {
  let mut lock = GENERATORS.lock();
  if lock.is_none() {
    *lock = Some(HashMap::new());
  }
  let map = lock.as_mut().unwrap();
  map.insert(name.into(), Box::new(func));
}

#[no_mangle]
extern "C" fn generate_chunk_and_lock(name: *const i8, x: i32, z: i32) -> *const u8 {
  let generator_name = unsafe { std::ffi::CStr::from_ptr(name) };
  let mut sections = vec![];
  let chunk = if let Some(map) = GENERATORS.lock().as_ref() {
    if let Some(gen) = map.get(generator_name.to_str().unwrap()) {
      let mut chunk = Chunk::<paletted::Section>::new(8);
      gen(&mut chunk, ChunkPos::new(x, z));
      chunk
    } else {
      return 0 as _;
    }
  } else {
    return 0 as _;
  };
  for section in chunk.sections().flatten() {
    sections.push(section);
  }
  let mut lock = CHUNK_BUF.lock();
  if lock.is_none() {
    *lock = Some(vec![0; 8192 * 16]);
  }
  let buffer = lock.as_mut().unwrap();
  let mut writer = MessageWriter::new(buffer);
  writer.write(&sections).unwrap();
  let ptr = buffer.as_ptr();
  std::mem::forget(lock);
  ptr
}
#[no_mangle]
extern "C" fn tick() {}

#[no_mangle]
extern "C" fn unlock_generated_chunk() {
  // SAFETY: The caller must call generate_chunk_and_lock before this, where
  // we call `mem::forget` on a `BUF` lock.
  unsafe {
    CHUNK_BUF.force_unlock();
  }
}

#[no_mangle]
extern "C" fn malloc(size: u32, align: u32) -> u32 {
  use std::alloc::{alloc, Layout};
  unsafe { alloc(Layout::from_size_align(size as usize, align as usize).unwrap()) as u32 }
}
#[no_mangle]
extern "C" fn free(ptr: u32, size: u32, align: u32) {
  use std::alloc::{dealloc, Layout};
  unsafe { dealloc(ptr as _, Layout::from_size_align(size as usize, align as usize).unwrap()) }
}

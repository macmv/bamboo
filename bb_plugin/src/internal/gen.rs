use bb_common::{
  chunk::{paletted, Chunk},
  math::ChunkPos,
  transfer::MessageWriter,
};
use parking_lot::{lock_api::RawMutex, Mutex};
use std::collections::HashMap;

static CHUNK_BUF: Mutex<Option<Vec<u8>>> = Mutex::const_new(RawMutex::INIT, None);
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

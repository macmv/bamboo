use crate::sync::{ConstLock, LazyLock};
use bb_common::{
  chunk::{paletted, Chunk},
  math::ChunkPos,
  transfer::MessageWriter,
};
use std::collections::HashMap;

type GeneratorFn = Box<dyn Fn(&mut Chunk<paletted::Section>, ChunkPos) + Send>;

static CHUNK_BUF: ConstLock<Vec<u8>> = ConstLock::new(vec![]);
static GENERATORS: LazyLock<HashMap<String, GeneratorFn>> = LazyLock::new(|| HashMap::new());

pub fn add_world_generator(
  name: &str,
  func: impl Fn(&mut Chunk<paletted::Section>, ChunkPos) + Send + 'static,
) {
  let mut map = GENERATORS.lock();
  map.insert(name.into(), Box::new(func));
}

#[no_mangle]
extern "C" fn generate_chunk_and_lock(name: *const i8, x: i32, z: i32) -> *const u8 {
  let generator_name = unsafe { std::ffi::CStr::from_ptr(name as _) };
  let mut sections = vec![];
  let map = GENERATORS.lock();
  let chunk = if let Some(gen) = map.get(generator_name.to_str().unwrap()) {
    let mut chunk = Chunk::<paletted::Section>::new(8);
    gen(&mut chunk, ChunkPos::new(x, z));
    chunk
  } else {
    return 0 as _;
  };
  for section in chunk.sections().flatten() {
    sections.push(section);
  }
  let mut buffer = CHUNK_BUF.lock();
  buffer.clear();
  let mut writer = MessageWriter::<&mut Vec<u8>>::new(&mut buffer);
  writer.write(&sections).unwrap();
  let ptr = buffer.as_ptr();
  std::mem::forget::<crate::sync::ConstGuard<_>>(buffer);
  ptr
}

#[no_mangle]
extern "C" fn unlock_generated_chunk() {
  // SAFETY: The caller must call generate_chunk_and_lock before this, where
  // we call `mem::forget` on a `BUF` lock.
  unsafe {
    CHUNK_BUF.force_unlock();
  }
}

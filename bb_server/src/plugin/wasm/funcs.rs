use crate::world::WorldManager;
use bb_common::util::Chat;
use bb_ffi::CChat;
use std::sync::Arc;
use wasmer::{imports, Function, ImportObject, LazyInit, Memory, Store, WasmPtr, WasmerEnv, Array};

#[derive(WasmerEnv, Clone)]
pub struct Env {
  #[wasmer(export)]
  pub memory: LazyInit<Memory>,
  pub wm:     Arc<WorldManager>,
  pub name:   Arc<String>,
}

impl Env {
  pub fn mem(&self) -> &Memory { self.memory.get_ref().expect("Env not initialized") }
}

fn info(env: &Env, message: WasmPtr<u8, Array>) {
  unsafe {
    let s = message.get_utf8_str_with_nul(env.mem()).unwrap();
    info!("`{}`: {}", env.name, s);
  }
}

fn broadcast(env: &Env, message: WasmPtr<CChat>) {
  let chat = message.deref(env.mem()).unwrap().get();
  let ptr = WasmPtr::<u8, _>::new(chat.message as u32);
  let s = ptr.get_utf8_string_with_nul(env.mem()).unwrap();
  env.wm.broadcast(Chat::new(s));
}

fn player_username(env: &Env, player: i32, buf: WasmPtr<u8>, buf_len: u32) -> i32 {
  let player = match env.wm.get_player(bb_common::util::UUID::from_u128(0)) {
    Some(p) => p,
    None => return 1,
  };
  let bytes = player.username().as_bytes();
  let end = buf.offset() + bytes.len() as u32;
  if bytes.len() > buf_len as usize {
    return 1;
  }
  let mem = env.mem();
  if end as usize > mem.size().bytes().0 {
    return 1;
  }
  unsafe {
    let ptr = mem.view::<u8>().as_ptr().add(buf.offset() as usize) as *mut u8;
    let slice: &mut [u8] = std::slice::from_raw_parts_mut(ptr, bytes.len());
    slice.copy_from_slice(bytes);
  }
  0
}

pub fn imports(store: &Store, wm: Arc<WorldManager>, name: String) -> ImportObject {
  let env = Env { memory: LazyInit::new(), wm, name: Arc::new(name) };
  imports! {
    "env" => {
      "bb_info" => Function::new_native_with_env(&store, env.clone(), info),
      "bb_broadcast" => Function::new_native_with_env(&store, env.clone(), broadcast),
      "bb_player_username" => Function::new_native_with_env(&store, env.clone(), player_username),
    }
  }
}

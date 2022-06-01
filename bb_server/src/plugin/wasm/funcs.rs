use crate::{
  block,
  command::{Command, NodeType, Parser},
  world::WorldManager,
};
use bb_common::{math::Pos, util::Chat, version::BlockVersion};
use bb_ffi::{CChat, CCommand, CPos, CUUID};
use log::Level;
use std::sync::Arc;
use wasmer::{imports, Array, Function, ImportObject, LazyInit, Memory, Store, WasmPtr, WasmerEnv};

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

fn log_from_level(level: u32) -> Option<Level> {
  Some(match level {
    1 => Level::Error,
    2 => Level::Warn,
    3 => Level::Info,
    4 => Level::Debug,
    5 => Level::Trace,
    _ => return None,
  })
}

fn log(
  env: &Env,
  level: u32,
  message_ptr: WasmPtr<u8, Array>,
  message_len: u32,
  target_ptr: WasmPtr<u8, Array>,
  target_len: u32,
  module_path_ptr: WasmPtr<u8, Array>,
  module_path_len: u32,
  file_ptr: WasmPtr<u8, Array>,
  file_len: u32,
  line: u32,
) {
  let level = match log_from_level(level) {
    Some(l) => l,
    None => return,
  };
  // SAFETY: We aren't using the string outside this function,
  // so this is safe. It also avoids allocating, so that's why
  // we use this instead of `get_utf8_string_with_nul`.

  unsafe {
    log::logger().log(
      &log::Record::builder()
        .args(format_args!("{}", message_ptr.get_utf8_str(env.mem(), message_len).unwrap()))
        .level(level)
        .target(target_ptr.get_utf8_str(env.mem(), target_len).unwrap())
        .module_path(Some(module_path_ptr.get_utf8_str(env.mem(), module_path_len).unwrap()))
        .file(Some(file_ptr.get_utf8_str(env.mem(), file_len).unwrap()))
        .line(Some(line))
        .build(),
    );
  }
}

fn broadcast(env: &Env, message: WasmPtr<CChat>) {
  let chat = message.deref(env.mem()).unwrap().get();
  let s = chat.message.ptr.get_utf8_string_with_nul(env.mem()).unwrap();
  env.wm.broadcast(Chat::new(s));
}

fn player_username(env: &Env, id: WasmPtr<CUUID>, buf: WasmPtr<u8>, buf_len: u32) -> i32 {
  let mem = env.mem();
  let uuid = match id.deref(mem) {
    Some(id) => id.get(),
    None => return 1,
  };
  let player = match env.wm.get_player(bb_common::util::UUID::from_u128(
    (uuid.bytes[3] as u128) << (3 * 32)
      | (uuid.bytes[2] as u128) << (2 * 32)
      | (uuid.bytes[1] as u128) << 32
      | uuid.bytes[0] as u128,
  )) {
    Some(p) => p,
    None => return 1,
  };
  let bytes = player.username().as_bytes();
  let end = buf.offset() + bytes.len() as u32;
  if bytes.len() > buf_len as usize {
    return 1;
  }
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

fn player_world(env: &Env, player: WasmPtr<CUUID>) -> i32 {
  let mem = env.mem();
  let uuid = match player.deref(mem) {
    Some(p) => p.get(),
    None => return -1,
  };
  let _player = match env.wm.get_player(bb_common::util::UUID::from_u128(
    (uuid.bytes[3] as u128) << (3 * 32)
      | (uuid.bytes[2] as u128) << (2 * 32)
      | (uuid.bytes[1] as u128) << 32
      | uuid.bytes[0] as u128,
  )) {
    Some(p) => p,
    None => return -1,
  };
  0
}

fn world_set_block(env: &Env, wid: u32, pos: WasmPtr<CPos>, id: u32, version: u32) -> i32 {
  let mem = env.mem();
  let pos = match pos.deref(mem) {
    Some(p) => p.get(),
    None => return -1,
  };
  let world = env.wm.default_world();
  let ty = env.wm.block_converter().type_from_id(id, BlockVersion::latest());
  match world.set_block(Pos::new(pos.x, pos.y, pos.z), ty) {
    Ok(_) => 0,
    Err(_) => -1,
  }
}

fn add_command(env: &Env, cmd: WasmPtr<CCommand>) {
  fn command_from_env(env: &Env, cmd: WasmPtr<CCommand>) -> Option<Command> {
    unsafe {
      let mem = env.mem();
      let cmd = match cmd.deref(mem) {
        Some(c) => c.get(),
        None => return None,
      };
      let name = cmd.name.ptr.get_utf8_str(mem, cmd.name.len)?.into();
      let _parser = cmd.parser.ptr.get_utf8_str(mem, cmd.parser.len)?;
      let ty = match cmd.node_type {
        0 => NodeType::Literal,
        1 => NodeType::Argument(Parser::BlockPos),
        _ => return None,
      };
      let mut children = Vec::with_capacity(cmd.children.len as usize);
      for i in 0..cmd.children.len {
        children
          .push(command_from_env(env, WasmPtr::new(cmd.children.get_ptr(i).unwrap() as u32))?);
      }

      Some(Command::new_from_plugin(name, ty, children, cmd.optional == 1))
    }
  }
  if let Some(cmd) = command_from_env(env, cmd) {
    env.wm.commands().add(cmd, |_, _, _| {});
  }
}

fn time_since_start(env: &Env) -> u64 {
  use parking_lot::{lock_api::RawMutex, Mutex};
  use std::time::Instant;

  static START: Mutex<Option<Instant>> = Mutex::const_new(parking_lot::RawMutex::INIT, None);

  let mut lock = START.lock();
  match *lock {
    Some(start) => start.elapsed().as_nanos() as u64,
    None => {
      *lock = Some(Instant::now());
      0
    }
  }
}

pub fn imports(store: &Store, wm: Arc<WorldManager>, name: String) -> ImportObject {
  let env = Env { memory: LazyInit::new(), wm, name: Arc::new(name) };
  imports! {
    "env" => {
      "bb_log" => Function::new_native_with_env(&store, env.clone(), log),
      "bb_broadcast" => Function::new_native_with_env(&store, env.clone(), broadcast),
      "bb_player_username" => Function::new_native_with_env(&store, env.clone(), player_username),
      "bb_player_world" => Function::new_native_with_env(&store, env.clone(), player_world),
      "bb_world_set_block" => Function::new_native_with_env(&store, env.clone(), world_set_block),
      "bb_add_command" => Function::new_native_with_env(&store, env.clone(), add_command),
      "bb_time_since_start" => Function::new_native_with_env(&store, env.clone(), time_since_start),
    }
  }
}

use super::{FromFfi, ToFfi};
use crate::{
  block,
  command::{Command, NodeType, Parser},
  particle::Particle,
  world::WorldManager,
};
use bb_common::{
  math::{FPos, Pos},
  util::Chat,
  version::BlockVersion,
};
use bb_ffi::{CBlockPropValue, CChat, CCommand, CCommandArg, CFPos, CList, CParticle, CPos, CUUID};
use log::Level;
use std::{mem, sync::Arc};
use wasmer::{
  imports, Array, Function, ImportObject, LazyInit, Memory, NativeFunc, Store, WasmPtr, WasmerEnv,
};

type OnCommand = NativeFunc<(WasmPtr<CUUID>, WasmPtr<CList<CCommandArg>>), ()>;
type WasmMalloc = NativeFunc<(u32, u32), u32>;

#[derive(WasmerEnv, Clone)]
pub struct Env {
  #[wasmer(export)]
  pub memory:      LazyInit<Memory>,
  #[wasmer(export)]
  pub wasm_malloc: LazyInit<WasmMalloc>,
  #[wasmer(export)]
  pub on_command:  LazyInit<OnCommand>,
  pub wm:          Arc<WorldManager>,
  /// The version of this plugin. Plugins will send us things like block ids,
  /// and we need to know how to convert them to the server's version. This
  /// allows us to load out-of-date plugins on a newer server.
  pub ver:         BlockVersion,
  pub name:        Arc<String>,
}

impl Env {
  pub fn mem(&self) -> &Memory { self.memory.get_ref().expect("Env not initialized") }
  pub fn malloc<T: Copy>(&self) -> WasmPtr<T> {
    let ptr = self
      .wasm_malloc
      .get_ref()
      .expect("Env not initialized")
      .call(mem::size_of::<T>() as u32, mem::align_of::<T>() as u32)
      .unwrap_or_else(|e| panic!("{e}"));
    WasmPtr::new(ptr)
  }
  pub fn malloc_array<T: Copy>(&self, len: u32) -> WasmPtr<T, Array> {
    let ptr = self
      .wasm_malloc
      .get_ref()
      .expect("Env not initialized")
      .call(mem::size_of::<T>() as u32 * len, mem::align_of::<T>() as u32)
      .unwrap();
    WasmPtr::new(ptr)
  }
  pub fn malloc_store<T: Copy>(&self, value: T) -> WasmPtr<T> {
    let ptr = self.malloc::<T>();
    // This checks that the last byte in our allocated region is valid for writes.
    if u64::from(ptr.offset()) + mem::size_of::<T>() as u64 - 1 > self.mem().data_size() {
      panic!("invalid ptr");
    }
    // SAFETY: We just validated the `write` call will write to valid memory.
    unsafe {
      let ptr = self.mem().data_ptr().add(ptr.offset() as usize);
      std::ptr::write(ptr as *mut T, value);
    }
    ptr
  }
  pub fn malloc_array_store<T: Copy>(&self, value: &[T]) -> WasmPtr<T, Array> {
    let ptr = self.malloc_array::<T>(value.len().try_into().unwrap());
    // This checks that the last byte in the array is within out data.
    if u64::from(ptr.offset()) + mem::size_of::<T>() as u64 * value.len() as u64 - 1
      > self.mem().data_size()
    {
      panic!("invalid ptr");
    }
    // SAFETY: We just validated the `write` call will write to valid memory.
    unsafe {
      // We want to call add on *mut u8, because ptr.offset() gives bytes.
      let ptr = self.mem().data_ptr().add(ptr.offset() as usize) as *mut T;
      std::ptr::copy(value.as_ptr(), ptr, value.len());
    }
    ptr
  }
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

#[allow(clippy::too_many_arguments)]
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
    let s = if u64::from(message_ptr.offset() + message_len) > env.mem().data_size() {
      std::borrow::Cow::Borrowed("")
    } else {
      let ptr = env.mem().data_ptr().add(message_ptr.offset() as usize) as *const u8;
      let slice = std::slice::from_raw_parts(ptr, message_len as usize);
      String::from_utf8_lossy(slice)
    };
    log::logger().log(
      &log::Record::builder()
        .args(format_args!("{s}"))
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

fn player_username(env: &Env, id: WasmPtr<CUUID>) -> u32 {
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
  let cusername = player.username().as_str().to_ffi(env);
  let ptr = env.malloc_store(cusername);
  ptr.offset()
}
fn player_pos(env: &Env, id: WasmPtr<CUUID>) -> u32 {
  let mem = env.mem();
  let uuid = match id.deref(mem) {
    Some(id) => id.get(),
    None => return 0,
  };
  let player = match env.wm.get_player(bb_common::util::UUID::from_u128(
    (uuid.bytes[3] as u128) << (3 * 32)
      | (uuid.bytes[2] as u128) << (2 * 32)
      | (uuid.bytes[1] as u128) << 32
      | uuid.bytes[0] as u128,
  )) {
    Some(p) => p,
    None => return 0,
  };
  let cpos = player.pos().to_ffi(env);
  let ptr = env.malloc_store(cpos);
  ptr.offset()
}
fn player_look_as_vec(env: &Env, id: WasmPtr<CUUID>) -> u32 {
  let mem = env.mem();
  let uuid = match id.deref(mem) {
    Some(id) => id.get(),
    None => return 0,
  };
  let player = match env.wm.get_player(bb_common::util::UUID::from_u128(
    (uuid.bytes[3] as u128) << (3 * 32)
      | (uuid.bytes[2] as u128) << (2 * 32)
      | (uuid.bytes[1] as u128) << 32
      | uuid.bytes[0] as u128,
  )) {
    Some(p) => p,
    None => return 0,
  };
  let cpos = FPos::from(player.look_as_vec()).to_ffi(env);
  let ptr = env.malloc_store(cpos);
  ptr.offset()
}
fn player_send_particle(env: &Env, id: WasmPtr<CUUID>, particle: WasmPtr<CParticle>) {
  let mem = env.mem();
  let uuid = match id.deref(mem) {
    Some(id) => id.get(),
    None => return,
  };
  let player = match env.wm.get_player(bb_common::util::UUID::from_u128(
    (uuid.bytes[3] as u128) << (3 * 32)
      | (uuid.bytes[2] as u128) << (2 * 32)
      | (uuid.bytes[1] as u128) << 32
      | uuid.bytes[0] as u128,
  )) {
    Some(p) => p,
    None => return,
  };
  let cparticle = match particle.deref(mem) {
    Some(p) => p.get(),
    None => return,
  };
  let particle = Particle::from_ffi(env, cparticle);
  player.send_particle(particle);
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

fn world_set_block(env: &Env, _wid: u32, pos: WasmPtr<CPos>, id: u32) -> i32 {
  let mem = env.mem();
  let pos = match pos.deref(mem) {
    Some(p) => p.get(),
    None => return -1,
  };
  let world = env.wm.default_world();
  let ty = env.wm.block_converter().type_from_id(id, env.ver);
  match world.set_block(Pos::new(pos.x, pos.y, pos.z), ty) {
    Ok(_) => 0,
    Err(_) => -1,
  }
}
fn world_set_block_kind(env: &Env, _wid: u32, pos: WasmPtr<CPos>, kind: u32) -> i32 {
  let mem = env.mem();
  let pos = match pos.deref(mem) {
    Some(p) => p.get(),
    None => return -1,
  };
  let world = env.wm.default_world();
  let kind = block::Kind::from_id(kind).unwrap_or(block::Kind::Air);
  match world.set_kind(Pos::new(pos.x, pos.y, pos.z), kind) {
    Ok(_) => 0,
    Err(_) => -1,
  }
}
fn world_get_block(env: &Env, _wid: u32, pos: WasmPtr<CPos>) -> u32 {
  let mem = env.mem();
  let pos = match pos.deref(mem) {
    Some(p) => p.get(),
    None => return u32::MAX,
  };
  let world = env.wm.default_world();
  match world.get_block(Pos::new(pos.x, pos.y, pos.z)) {
    Ok(ty) => ty.id(),
    Err(_) => u32::MAX,
  }
}
fn world_players(env: &Env, _wid: u32) -> u32 {
  let world = env.wm.default_world();
  let players: Vec<_> = world.players().iter().map(|p| p.id()).collect();
  let cplayers = players.as_slice().to_ffi(env);
  let ptr = env.malloc_store(cplayers);
  ptr.offset()
}
fn world_raycast(env: &Env, from: WasmPtr<CFPos>, to: WasmPtr<CFPos>, water: u8) -> u32 {
  let mem = env.mem();
  let from = match from.deref(mem) {
    Some(p) => FPos::from_ffi(env, p.get()),
    None => return 0,
  };
  let to = match to.deref(mem) {
    Some(p) => FPos::from_ffi(env, p.get()),
    None => return 0,
  };
  let water = water == 1;
  let world = env.wm.default_world();
  match world.raycast(from, to, water) {
    Some((pos, _res)) => {
      let cpos = pos.to_ffi(env);
      let ptr = env.malloc_store(cpos);
      ptr.offset()
    }
    None => 0,
  }
}
fn block_data_for_kind(env: &Env, kind: u32) -> u32 {
  // TODO: Convert kind to server version
  let data = env.wm.block_converter().get(match block::Kind::from_id(kind) {
    Some(id) => id,
    None => return 0,
  });
  let cdata = data.to_ffi(env);
  env.malloc_store(cdata).offset()
}
fn block_kind_for_type(env: &Env, ty: u32) -> u32 {
  let kind = env.wm.block_converter().kind_from_id(ty, env.ver);
  // TODO: Convert kind to plugin version
  kind.id()
}
fn block_prop(env: &Env, ty: u32, name_ptr: WasmPtr<u8, Array>, name_len: u32) -> u32 {
  let ty = env.wm.block_converter().type_from_id(ty, env.ver);
  let name = unsafe { name_ptr.get_utf8_str(env.mem(), name_len).unwrap() };
  match ty.try_prop(name) {
    Ok(prop) => {
      let cprop = prop.to_ffi(env);
      let ptr = env.malloc_store(cprop);
      ptr.offset()
    }
    Err(e) => {
      warn!("plugin failed to lookup property: {e}");
      0
    }
  }
}
fn block_set_prop(
  env: &Env,
  ty: u32,
  name_ptr: WasmPtr<u8, Array>,
  name_len: u32,
  prop: WasmPtr<CBlockPropValue>,
) -> u32 {
  let mut ty = env.wm.block_converter().type_from_id(ty, env.ver);
  let name = unsafe { name_ptr.get_utf8_str(env.mem(), name_len).unwrap() };
  let mem = env.mem();
  let prop = match prop.deref(mem) {
    Some(p) => p.get(),
    None => return ty.id(),
  };
  match ty.try_set_prop(name, &block::PropValueStore::from_ffi(env, prop)) {
    Ok(()) => env.wm.block_converter().to_old(ty.id(), env.ver),
    Err(e) => {
      error!("plugin tried to set invalid property: {e}");
      ty.id()
    }
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
      let parser = <Option<Parser>>::from_ffi(env, cmd.parser);
      let ty = match cmd.node_type {
        0 => NodeType::Literal,
        1 => NodeType::Argument(parser.unwrap()),
        _ => return None,
      };
      let mut children = Vec::with_capacity(cmd.children.len as usize);
      for i in 0..cmd.children.len {
        children
          .push(command_from_env(env, WasmPtr::new(cmd.children.get_ptr(i).unwrap() as u32))?);
      }

      Some(Command::new_from_plugin(name, ty, children, cmd.optional.as_bool()))
    }
  }
  if let Some(cmd) = command_from_env(env, cmd) {
    let e = env;
    let env = env.clone();
    e.wm.commands().add(cmd, move |_, player, args| {
      let id = match player {
        Some(p) => env.malloc_store(p.id().to_ffi(&env)),
        None => WasmPtr::new(0),
      };
      let args = env.malloc_store(args.as_slice().to_ffi(&env));
      match env.on_command.get_ref().unwrap().call(id, args) {
        Ok(()) => {}
        Err(e) => error!("couldn't execute command on wasm: {e}"),
      }
    });
  }
}

fn time_since_start(_env: &Env) -> u64 {
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
  let env = Env {
    memory: LazyInit::new(),
    wasm_malloc: LazyInit::new(),
    on_command: LazyInit::new(),
    wm,
    // TODO: Fetch this from the plugin
    ver: BlockVersion::latest(),
    name: Arc::new(name),
  };
  imports! {
    "env" => {
      "bb_log" => Function::new_native_with_env(store, env.clone(), log),
      "bb_add_command" => Function::new_native_with_env(store, env.clone(), add_command),
      "bb_block_data_for_kind" => Function::new_native_with_env(store, env.clone(), block_data_for_kind),
      "bb_block_kind_for_type" => Function::new_native_with_env(store, env.clone(), block_kind_for_type),
      "bb_block_prop" => Function::new_native_with_env(store, env.clone(), block_prop),
      "bb_block_set_prop" => Function::new_native_with_env(store, env.clone(), block_set_prop),
      "bb_broadcast" => Function::new_native_with_env(store, env.clone(), broadcast),
      "bb_player_username" => Function::new_native_with_env(store, env.clone(), player_username),
      "bb_player_pos" => Function::new_native_with_env(store, env.clone(), player_pos),
      "bb_player_look_as_vec" => Function::new_native_with_env(store, env.clone(), player_look_as_vec),
      "bb_player_world" => Function::new_native_with_env(store, env.clone(), player_world),
      "bb_player_send_particle" => Function::new_native_with_env(store, env.clone(), player_send_particle),
      "bb_world_set_block" => Function::new_native_with_env(store, env.clone(), world_set_block),
      "bb_world_set_block_kind" => Function::new_native_with_env(store, env.clone(), world_set_block_kind),
      "bb_world_get_block" => Function::new_native_with_env(store, env.clone(), world_get_block),
      "bb_world_players" => Function::new_native_with_env(store, env.clone(), world_players),
      "bb_world_raycast" => Function::new_native_with_env(store, env.clone(), world_raycast),
      "bb_time_since_start" => Function::new_native_with_env(store, env, time_since_start),
    }
  }
}

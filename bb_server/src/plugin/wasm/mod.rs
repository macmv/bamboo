mod input;
mod output;

use super::{PluginImpl, ServerEvent, ServerMessage};
use crate::world::WorldManager;
use bb_common::util::Chat;
use bb_ffi::CChat;
use std::{error::Error, fs, path::Path, sync::Arc};
use wasmer::{
  imports, ExportError, Function, Instance, LazyInit, Memory, Module, NativeFunc, Store, WasmPtr,
  WasmTypeList, WasmerEnv,
};

pub struct Plugin {
  inst: Instance,
}

/// This is the last argument to every wasm function. It is the pointer type (we
/// are using 32 bit), and it is used for the function to write it's result
/// into.
type OUT = u32;

trait Input {
  type WasmArgs: WasmTypeList;
  /// Calls native, and passes the pointer as the last argument.
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Option<Rets>;
}
trait Output {
  /// Returns the size in pointers. This keeps everything aligned, and means we
  /// use `MemoryView<i32>` everywhere.
  fn size() -> u32;
  /// Returns `Self`, given the address in pointers.
  fn from_addr(mem: &Memory, addr: OUT) -> Self;
}

#[derive(WasmerEnv, Clone)]
pub struct Env {
  #[wasmer(export)]
  memory: LazyInit<Memory>,
  wm:     Arc<WorldManager>,
}

fn broadcast(env: &Env, message: WasmPtr<CChat>) {
  let chat = message.deref(env.mem()).unwrap().get();
  let ptr = WasmPtr::<u8, _>::new(chat.message as u32);
  let s = ptr.get_utf8_string_with_nul(env.mem()).unwrap();
  env.wm.broadcast(Chat::new(s));
}

fn player_username(env: &Env, player: i32, buf: WasmPtr<u8>, buf_len: u32) {
  let player = env.wm.get_player(bb_common::util::UUID::from_u128(0)).unwrap();
  let bytes = player.username().as_bytes();
  let end = buf.offset() + bytes.len() as u32;
  if bytes.len() > buf_len as usize {
    return;
  }
  let mem = env.mem();
  if end as usize > mem.size().bytes().0 {
    return;
  }
  unsafe {
    let ptr = mem.view::<u8>().as_ptr().add(buf.offset() as usize) as *mut u8;
    let slice: &mut [u8] = std::slice::from_raw_parts_mut(ptr, bytes.len());
    slice.copy_from_slice(bytes);
  }
}

impl Env {
  pub fn mem(&self) -> &Memory { self.memory.get_ref().expect("Env not initialized") }
}

impl Plugin {
  pub fn new(
    _name: String,
    path: &Path,
    output: String,
    wm: Arc<WorldManager>,
  ) -> Result<Self, Box<dyn Error>> {
    let store = Store::default();
    let module = Module::new(&store, fs::read(path.join(output))?)?;
    let env = Env { memory: LazyInit::new(), wm };
    let import_object = imports! {
      "env" => {
        "broadcast" => Function::new_native_with_env(&store, env.clone(), broadcast),
        "player_username" => Function::new_native_with_env(&store, env.clone(), player_username),
      }
    };
    let inst = Instance::new(&module, &import_object)?;
    Ok(Plugin { inst })
  }

  fn call<I: Input>(&self, name: &str, input: I) -> Result<bool, ()> {
    // Try to get function with bool. If this fails, try with no return. If that
    // fails, error out.
    //
    // If the function doesn't exist, we ignore the error.
    match self.inst.exports.get_native_function::<I::WasmArgs, u8>(name) {
      Ok(func) => Ok(input.call_native(&func).unwrap() != 0),
      Err(ExportError::IncompatibleType) => {
        match self.inst.exports.get_native_function::<I::WasmArgs, ()>(name) {
          Ok(func) => {
            input.call_native(&func);
            Ok(true)
          }
          Err(ExportError::IncompatibleType) => {
            error!("incompatible types when calling {name}");
            Err(())
          }
          Err(ExportError::Missing(_)) => Ok(true),
        }
      }
      Err(ExportError::Missing(_)) => Ok(true),
    }
  }
}

impl PluginImpl for Plugin {
  fn call(&self, m: ServerMessage) -> Result<bool, ()> {
    Ok(match m {
      ServerMessage::Event { player, event } => match event {
        ServerEvent::BlockPlace { pos, .. } => {
          self.call("on_block_place", (player.eid(), pos.x(), pos.y(), pos.z()))?
        }
        _ => true,
      },
      _ => true,
    })
  }
}

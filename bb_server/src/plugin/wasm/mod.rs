mod funcs;
mod input;
mod output;

use super::{CallError, PluginImpl, ServerEvent, ServerMessage};
use crate::world::WorldManager;
use bb_ffi::CUUID;
use std::{error::Error, fs, path::Path, sync::Arc};
use wasmer::{ExportError, Instance, Memory, Module, NativeFunc, Store, WasmTypeList};

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

impl Plugin {
  pub fn new(
    name: String,
    path: &Path,
    output: String,
    wm: Arc<WorldManager>,
  ) -> Result<Self, Box<dyn Error>> {
    let store = Store::default();
    let module = Module::new(&store, fs::read(path.join(output))?)?;
    let import_object = funcs::imports(&store, wm, name);
    let inst = Instance::new(&module, &import_object)?;
    let plug = Plugin { inst };
    plug.call("init", ()).unwrap();
    Ok(plug)
  }

  fn call<I: Input>(&self, name: &str, input: I) -> Result<bool, CallError> {
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
          Err(e @ ExportError::IncompatibleType) => Err(CallError::no_keep(e)),
          Err(ExportError::Missing(_)) => Ok(true),
        }
      }
      Err(ExportError::Missing(_)) => Ok(true),
    }
  }
}

impl PluginImpl for Plugin {
  fn call(&self, m: ServerMessage) -> Result<bool, CallError> {
    Ok(match m {
      ServerMessage::Event { player, event } => match event {
        ServerEvent::BlockPlace { pos, .. } => self.call(
          "on_block_place",
          (
            CUUID {
              bytes: [
                u32::from_ne_bytes(player.id().as_le_bytes()[0..4].try_into().unwrap()),
                u32::from_ne_bytes(player.id().as_le_bytes()[4..8].try_into().unwrap()),
                u32::from_ne_bytes(player.id().as_le_bytes()[8..12].try_into().unwrap()),
                u32::from_ne_bytes(player.id().as_le_bytes()[12..16].try_into().unwrap()),
              ],
            },
            pos.x(),
            pos.y(),
            pos.z(),
          ),
        )?,
        _ => true,
      },
      _ => true,
    })
  }
}

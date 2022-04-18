mod input;
mod output;

use super::{PluginImpl, ServerMessage};
use std::{error::Error, fs, path::Path};
use wasmer::{
  imports, Function, HostEnvInitError, Instance, LazyInit, Memory, Module, NativeFunc, Store,
  WasmPtr, WasmTypeList, WasmerEnv,
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
    addr: OUT,
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
}

fn broadcast(env: &Env, message: i32) {
  let ptr = WasmPtr::<u8, _>::new(message as u32);
  let s = ptr.get_utf8_string_with_nul(env.memory.get_ref().unwrap()).unwrap();
  dbg!(s);
}

impl Plugin {
  pub fn new(name: String, path: &Path, output: String) -> Result<Self, Box<dyn Error>> {
    let store = Store::default();
    let module = Module::new(&store, fs::read(path.join(output))?)?;
    let env = Env { memory: LazyInit::new() };
    let import_object = imports! {
      "env" => {
        "broadcast" => Function::new_native_with_env(&store, env, broadcast),
      }
    };
    let inst = Instance::new(&module, &import_object)?;
    Ok(Plugin { inst })
  }

  fn call<I: Input, O: Output>(&self, name: &str, input: I) -> Option<O> {
    let mem = self.inst.exports.get_memory("memory").unwrap();
    // FIXME:
    // This seems stupid, but in what world is address zero valid? The plugin will
    // never allocate at address zero, and it probably won't write to it unless we
    // tell it to. Because I can't figure out a better solution, I'm just going to
    // leave it as is until its a problem.
    let out = 0_u32;
    let func = self.inst.exports.get_native_function::<I::WasmArgs, ()>(name).unwrap();
    input.call_native(&func, out).unwrap();
    Some(O::from_addr(mem, out))
  }
}

impl PluginImpl for Plugin {
  fn call(&self, ev: ServerMessage) -> Result<bool, ()> {
    let res = self.call::<(), ()>("init", ()).unwrap();
    dbg!(res);
    Ok(true)
  }
}

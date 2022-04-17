mod input;
mod output;

use super::{PluginImpl, ServerMessage};
use std::{error::Error, fs, path::Path};
use wasmer::{imports, Instance, Memory, Module, NativeFunc, Store, WasmTypeList};

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

impl Plugin {
  pub fn new(name: String, path: &Path, output: String) -> Result<Self, Box<dyn Error>> {
    let store = Store::default();
    let module = Module::new(&store, fs::read(path.join(output))?)?;
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
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
    let res = self.call::<i32, (bool, String)>("init", 4).unwrap();
    dbg!(res);
    Ok(true)
  }
}

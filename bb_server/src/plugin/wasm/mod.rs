use super::{PluginImpl, ServerMessage};
use std::{error::Error, fs, path::Path};
use wasmer::{imports, Instance, Memory, MemoryType, Module, Pages, Store, Value, WasmPtr};

pub struct Plugin {
  inst: Instance,
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
}

impl PluginImpl for Plugin {
  fn call(&self, ev: ServerMessage) -> Result<bool, ()> {
    if let Ok(add_one) = self.inst.exports.get_function("init") {
      let mem = self.inst.exports.get_memory("memory").unwrap();
      let out = 0;
      add_one.call(&[Value::I32(5), Value::I32(out)]).unwrap();
      let ptr = WasmPtr::<u8, _>::new(mem.view::<u32>()[out as usize].get());
      let len = mem.view::<u32>()[out as usize + 1].get();
      unsafe {
        dbg!(ptr.get_utf8_str(mem, len).unwrap());
      }
    }
    Ok(true)
  }
}

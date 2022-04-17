use super::{PluginImpl, ServerMessage};
use std::error::Error;
use wasmer::{imports, Instance, Module, Store, Value};

pub struct Plugin {
  inst: Instance,
}

impl Plugin {
  pub fn new(name: String) -> Result<Self, Box<dyn Error>> {
    let module_wat = r#"
      (module
        (type $t0 (func (param i32) (result i32)))
        (func $add_one (export "add_one") (type $t0) (param $p0 i32) (result i32)
          get_local $p0
          i32.const 1
          i32.add))
      "#;

    let store = Store::default();
    let module = Module::new(&store, &module_wat)?;
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
    let inst = Instance::new(&module, &import_object)?;
    Ok(Plugin { inst })
  }
}

impl PluginImpl for Plugin {
  fn call(&self, ev: ServerMessage) -> Result<bool, ()> {
    if let Ok(add_one) = self.inst.exports.get_function("add_one") {
      let res = add_one.call(&[Value::I32(42)]).unwrap();
      dbg!(res);
    }
    Ok(true)
  }
}

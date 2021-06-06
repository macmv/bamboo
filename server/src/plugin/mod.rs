mod plugin;

pub use plugin::Plugin;

use rutie::{Module, NilClass, Object, RString, VM};
use std::fs;

pub struct PluginManager {
  // Vector of module names
  plugins: Vec<Module>,
}

module!(Sugarcane);

methods!(
  Sugarcane,
  rtself,
  fn broadcast(v: RString) -> NilClass {
    info!("Brodcasting message: {}", v.unwrap().to_str());
    NilClass::new()
  },
);

impl PluginManager {
  pub fn new() -> Self {
    VM::init();

    Module::new("Sugarcane").define(|c| {
      c.def_self("broadcast", broadcast);
    });

    let mut m = PluginManager { plugins: vec![] };
    m.load();
    m
  }
  fn load(&mut self) {
    for f in fs::read_dir("plugins").unwrap() {
      let f = f.unwrap();
      let m = fs::metadata(f.path()).unwrap();
      if m.is_file() {
        let source = fs::read_to_string(f.path()).unwrap();

        let module = VM::eval(&source).unwrap();
        let module = module.try_convert_to::<Module>().unwrap();
        let big = module.const_get("BIG");
        dbg!(&big.try_convert_to::<RString>().unwrap().to_str());

        if module.respond_to("init") {
          module.protect_send("init", &[]).unwrap();
        }

        self.plugins.push(module);
      }
    }
  }
  // /// Creates the `sugarcane` ruby module. Used whenever plugins are
  // /// re-loaded.
  // fn create_module(&self) -> PyResult<&PyModule> {
  //   let sugarcane = PyModule::new(self.gil.python(), "sugarcane")?;
  //   sugarcane.add_function(wrap_pyfunction!(get_world, sugarcane)?)?;
  //   Ok(sugarcane)
  // }
  // fn init(&self) -> Result<(), PyErr> {
  //   let sugarcane = PluginManager::create_module(self.py)?;
  //   let mut plugins = self.plugins.lock().unwrap();
  //   plugins.clear();
  //
  //
  //   Ok(())
  // }
}

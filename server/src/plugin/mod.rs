mod plugin;

pub use plugin::Plugin;

use rutie::{Module, NilClass, Object, RString, VM};
use std::fs;

pub struct PluginManager {
  // Vector of module names
  plugins: Vec<Plugin>,
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
    self.plugins.clear();
    for f in fs::read_dir("plugins").unwrap() {
      let f = f.unwrap();
      let m = fs::metadata(f.path()).unwrap();
      if m.is_file() {
        let path = f.path();
        VM::require(&format!("./{}", path.to_str().unwrap()));

        // This converts plug.rb to Plug
        let name = path.file_stem().unwrap().to_str().unwrap();
        let name = name[..1].to_ascii_uppercase() + &name[1..];
        let module = Module::from_existing(&name);

        self.plugins.push(Plugin::new(name, module));
      }
    }
    for p in &self.plugins {
      p.init();
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

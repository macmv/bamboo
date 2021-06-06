mod plugin;

pub use plugin::Plugin;

use rutie::{Module, NilClass, Object, RString, VM};
use std::fs;

/// A struct that manages all Ruby plugins. This will handle re-loading all the
/// source files on `/reload`, and will also send events to all the plugins when
/// needed.
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
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  pub fn new() -> Self {
    VM::init();

    Module::new("Sugarcane").define(|c| {
      c.def_self("broadcast", broadcast);
    });

    let mut m = PluginManager { plugins: vec![] };
    m.load();
    m
  }

  /// Loads all plugins from disk. Call this to reload all plugins.
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
}

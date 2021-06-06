mod plugin;

pub use plugin::Plugin;

use crate::world::WorldManager;
use common::util::Chat;
use rutie::{types::Value, Module, NilClass, Object, RString, VM};
use std::{
  fs,
  sync::{Arc, Mutex},
};

/// A struct that manages all Ruby plugins. This will handle re-loading all the
/// source files on `/reload`, and will also send events to all the plugins when
/// needed.
pub struct PluginManager {
  // Vector of module names
  plugins: Mutex<Vec<Plugin>>,
}

#[repr(C)]
pub struct Sugarcane {
  value: Value,
  wm:    Option<Arc<WorldManager>>,
}

impl From<Value> for Sugarcane {
  fn from(value: Value) -> Self {
    Sugarcane { value, wm: None }
  }
}

impl Object for Sugarcane {
  #[inline]
  fn value(&self) -> Value {
    self.value
  }
}

methods!(
  Sugarcane,
  rtself,
  fn broadcast(v: RString) -> NilClass {
    let msg = Chat::new(v.unwrap().to_string());
    let wm = rtself.wm.unwrap();
    info!("Broadcasting?");
    tokio::task::block_in_place(|| {
      tokio::runtime::Handle::current().block_on(async move {
        wm.broadcast(&msg).await;
      })
    });

    NilClass::new()
  },
);

impl PluginManager {
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  pub fn new() -> Self {
    PluginManager { plugins: Mutex::new(vec![]) }
  }

  pub fn init(&self, wm: Arc<WorldManager>) {
    VM::init();

    let sc = Module::new("Sugarcane").define(|c| {
      c.define_nested_class("Sugarcane", None).define(|c| {
        c.define_method("broadcast", broadcast);
      });
    });

    self.load();
  }

  /// Loads all plugins from disk. Call this to reload all plugins.
  fn load(&self) {
    let mut plugins = self.plugins.lock().unwrap();
    plugins.clear();
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

        plugins.push(Plugin::new(name, module));
      }
    }
    for p in plugins.iter() {
      p.init();
    }
  }
}

mod plugin;
mod wrapper;

pub use plugin::Plugin;

use crate::{block, player::Player, world::WorldManager};
use common::math::Pos;
use rutie::{AnyException, Exception, Module, Object, RString, VM};
use std::{
  fs,
  sync::{mpsc, Arc, Mutex},
};
use wrapper::SugarcaneRb;

/// A struct that manages all Ruby plugins. This will handle re-loading all the
/// source files on `/reload`, and will also send events to all the plugins when
/// needed.
pub struct PluginManager {
  // Vector of module names
  plugins: Mutex<Vec<Plugin>>,
  tx:      Mutex<mpsc::Sender<()>>,
  rx:      Mutex<mpsc::Receiver<()>>,
}

impl PluginManager {
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  pub fn new() -> Self {
    let (tx, rx) = mpsc::channel();
    PluginManager { plugins: Mutex::new(vec![]), tx: Mutex::new(tx), rx: Mutex::new(rx) }
  }

  pub async fn run(&self, wm: Arc<WorldManager>) {
    VM::init();
    wrapper::create_module();
    self.load(wm);
    let rx = self.rx.lock().unwrap();
    loop {
      if let Ok(_) = rx.recv() {
        if let Ok(e) = VM::error_pop() {
          error!("{}", e.inspect());
          for l in e.backtrace().unwrap() {
            error!("{}", l.try_convert_to::<RString>().unwrap().to_str());
          }
        }
      }
    }
  }

  /// Loads all plugins from disk. Call this to reload all plugins.
  fn load(&self, wm: Arc<WorldManager>) {
    let mut plugins = self.plugins.lock().unwrap();
    let tx = self.tx.lock().unwrap();
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

        plugins.push(Plugin::new(name, module, tx.clone()));
      }
    }
    for p in plugins.iter() {
      p.init(SugarcaneRb::new(wm.clone()));
    }
  }

  pub fn on_block_place(&self, player: Arc<Player>, pos: Pos, kind: block::Kind) {
    for p in self.plugins.lock().unwrap().iter() {
      p.on_block_place(player.clone(), pos, kind);
    }
  }
}

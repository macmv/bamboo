mod plugin;

pub use plugin::Plugin;

use crate::{block, player::Player, world::WorldManager};
use common::math::Pos;
use std::sync::{mpsc, Arc, Mutex};

#[derive(Debug)]
pub enum Event {
  Init,
  OnBlockPlace(Arc<Player>, Pos, block::Kind),
}

/// A struct that manages all Ruby plugins. This will handle re-loading all the
/// source files on `/reload`, and will also send events to all the plugins when
/// needed.
pub struct PluginManager {
  // Vector of module names
  plugins: Mutex<Vec<Plugin>>,
  tx:      Mutex<mpsc::Sender<Event>>,
  rx:      Mutex<mpsc::Receiver<Event>>,
}

impl PluginManager {
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  pub fn new() -> Self {
    let (tx, rx) = mpsc::channel();
    PluginManager { plugins: Mutex::new(vec![]), tx: Mutex::new(tx), rx: Mutex::new(rx) }
  }

  pub async fn run(&self, wm: Arc<WorldManager>) {
    let rx = self.rx.lock().unwrap();
    self.handle_event(Event::Init);
    loop {
      if let Ok(e) = rx.recv() {
        self.handle_event(e);
      }
    }
  }

  fn handle_event(&self, e: Event) {
    info!("got event: {:?}", e);
  }

  //   /// Loads all plugins from disk. Call this to reload all plugins.
  //   fn load(&self, wm: Arc<WorldManager>) {
  //     let mut plugins = self.plugins.lock().unwrap();
  //     let tx = self.tx.lock().unwrap();
  //     plugins.clear();
  //     for f in fs::read_dir("plugins").unwrap() {
  //       let f = f.unwrap();
  //       let m = fs::metadata(f.path()).unwrap();
  //       if m.is_file() {
  //         let path = f.path();
  //         VM::require(&format!("./{}", path.to_str().unwrap()));
  //
  //         // This converts plug.rb to Plug
  //         let name = path.file_stem().unwrap().to_str().unwrap();
  //         let name = name[..1].to_ascii_uppercase() + &name[1..];
  //         let module = Module::from_existing(&name);
  //
  //         plugins.push(Plugin::new(name, module, tx.clone()));
  //       }
  //     }
  //     for p in plugins.iter() {
  //       p.init(SugarcaneRb::new(wm.clone()));
  //     }
  //   }

  pub fn on_block_place(&self, player: Arc<Player>, pos: Pos, kind: block::Kind) {
    self.tx.lock().unwrap().send(Event::OnBlockPlace(player, pos, kind)).unwrap();
  }
}

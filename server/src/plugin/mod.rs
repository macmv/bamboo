mod plugin;

pub use plugin::Plugin;

use crate::{block, player::Player, world::WorldManager};
use boa::{
  class::{Class, ClassBuilder},
  gc::{Finalize, Trace},
  Context, Result, Value,
};
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

#[derive(Debug)]
pub struct Gaming {
  val: String,
}

impl Gaming {
  fn info(_this: &Value, _args: &[Value], _ctx: &mut Context) -> Result<Value> {
    info!("hello");
    Ok(Value::Null)
  }
}

impl Finalize for Gaming {}
unsafe impl Trace for Gaming {
  unsafe fn trace(&self) {}
  unsafe fn root(&self) {}
  unsafe fn unroot(&self) {}

  fn finalize_glue(&self) {}
}

impl Class for Gaming {
  const NAME: &'static str = "Gaming";
  const LENGTH: usize = 1;

  fn constructor(_this: &Value, args: &[Value], ctx: &mut Context) -> Result<Self> {
    let val = args.get(0).cloned().unwrap_or_default().to_string(ctx)?;
    Ok(Gaming { val: val.to_string() })
  }

  fn init(class: &mut ClassBuilder) -> Result<()> {
    class.static_method("info", 0, Self::info);

    Ok(())
  }
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
    let mut ctx = Context::new();
    ctx.register_global_class::<Gaming>().unwrap();
    ctx.eval("Gaming.info();").unwrap();

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

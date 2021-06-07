mod plugin;

pub use plugin::Plugin;

use crate::{block, player::Player, world::WorldManager};
use boa::{
  class::{Class, ClassBuilder},
  gc::{Finalize, Trace},
  property::PropertyKey,
  Context, Result, Value,
};
use common::math::Pos;
use std::{
  fs,
  sync::{mpsc, Arc, Mutex},
};

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
pub struct Sugarcane {
  val: String,
}

impl Sugarcane {
  fn info(_this: &Value, args: &[Value], ctx: &mut Context) -> Result<Value> {
    info!("{}", args[0].to_string(ctx).unwrap());
    Ok(Value::Null)
  }
}

impl Finalize for Sugarcane {}
unsafe impl Trace for Sugarcane {
  unsafe fn trace(&self) {}
  unsafe fn root(&self) {}
  unsafe fn unroot(&self) {}

  fn finalize_glue(&self) {}
}

impl Class for Sugarcane {
  const NAME: &'static str = "Sugarcane";
  const LENGTH: usize = 1;

  fn constructor(_this: &Value, args: &[Value], ctx: &mut Context) -> Result<Self> {
    let val = args.get(0).cloned().unwrap_or_default().to_string(ctx)?;
    Ok(Sugarcane { val: val.to_string() })
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
    let mut ctx = Context::new();
    ctx.register_global_class::<Sugarcane>().unwrap();
    self.load(&mut ctx, wm);

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

  /// Loads all plugins from disk. Call this to reload all plugins.
  fn load(&self, mut ctx: &mut Context, wm: Arc<WorldManager>) {
    let mut plugins = self.plugins.lock().unwrap();
    let tx = self.tx.lock().unwrap();
    plugins.clear();
    for f in fs::read_dir("plugins").unwrap() {
      let f = f.unwrap();
      let m = fs::metadata(f.path()).unwrap();
      if m.is_file() {
        let path = f.path();
        let source = fs::read_to_string(path).unwrap();
        let res = ctx.eval(&source).unwrap();
        dbg!(res.get_field("init", &mut ctx).unwrap().to_object(&mut ctx).unwrap().call(
          &Value::Null,
          &[],
          &mut ctx
        ));
      }
    }
  }

  pub fn on_block_place(&self, player: Arc<Player>, pos: Pos, kind: block::Kind) {
    self.tx.lock().unwrap().send(Event::OnBlockPlace(player, pos, kind)).unwrap();
  }
}

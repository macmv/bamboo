mod plugin;

pub use plugin::Plugin;

use crate::{block, player::Player, world::WorldManager};
use boa::{
  class::{Class, ClassBuilder},
  gc::{Finalize, Trace},
  object::{Object, ObjectData},
  property::Attribute,
  Context, Result, Value,
};
use common::math::Pos;
use std::{
  fmt, fs,
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

pub struct Sugarcane {
  wm: Arc<WorldManager>,
}

impl fmt::Debug for Sugarcane {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Sugarcane {{}}")
  }
}

impl Sugarcane {
  fn new(wm: Arc<WorldManager>) -> Self {
    Sugarcane { wm }
  }
  fn info(_this: &Value, args: &[Value], ctx: &mut Context) -> Result<Value> {
    info!("{}", args[0].to_string(ctx).unwrap());
    Ok(Value::Null)
  }
  fn add_plugin(this: &Value, args: &[Value], ctx: &mut Context) -> Result<Value> {
    if let ObjectData::NativeObject(s) = &this.to_object(ctx)?.borrow().data {
      info!("got self: {:?}", s);
    } else {
      error!("gaming?");
    }
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

  fn constructor(_this: &Value, _args: &[Value], _ctx: &mut Context) -> Result<Self> {
    Err(Value::String("cannot construct Sugarcane from JS".into()))
  }

  fn init(class: &mut ClassBuilder) -> Result<()> {
    class.static_method("info", 0, Self::info);
    class.method("add_plugin", 0, Self::add_plugin);

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
    let o = ctx.construct_object();
    ctx.register_global_property(
      "sc",
      Object::native_object(Box::new(Sugarcane::new(wm.clone()))),
      Attribute::all(),
    );
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
        // let res = match ctx.eval(&source) {
        //   Ok(v) => v,
        //   Err(e) => {
        //     dbg!(&e);
        //     info!("{}", e.to_string(&mut ctx).unwrap());
        //     panic!()
        //   }
        // };
        // dbg!(v
        //   .get_field("init", &mut ctx)
        //   .unwrap()
        //   .to_object(&mut ctx)
        //   .unwrap()
        //   .call(&Value::Null, &[], &mut ctx)
        //   .unwrap());
      }
    }
  }

  pub fn on_block_place(&self, player: Arc<Player>, pos: Pos, kind: block::Kind) {
    self.tx.lock().unwrap().send(Event::OnBlockPlace(player, pos, kind)).unwrap();
  }
}

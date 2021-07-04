mod plugin;

pub use plugin::Plugin;

use crate::{block, player::Player, world::WorldManager};
use boa::{
  class::{Class, ClassBuilder},
  exec::Executable,
  gc::{Finalize, Trace},
  object::{Object, ObjectData},
  property::Attribute,
  syntax::parser::Parser,
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
    info!("{}", args.get(0).cloned().unwrap_or_default().to_string(ctx)?);
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
    let _o = ctx.construct_object();
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

  fn handle_event(&self, _e: Event) {}

  /// Loads all plugins from disk. Call this to reload all plugins.
  fn load(&self, _ctx: &mut Context, _wm: Arc<WorldManager>) {
    let mut plugins = self.plugins.lock().unwrap();
    let _tx = self.tx.lock().unwrap();
    plugins.clear();
    for f in fs::read_dir("plugins").unwrap() {
      let f = f.unwrap();
      let m = fs::metadata(f.path()).unwrap();
      if m.is_file() {
        let path = f.path();
        info!("found plugin at {}", path.to_str().unwrap());
        // let src = fs::read_to_string(path).unwrap();
        // let src_bytes = src.as_bytes();
        //
        // let parsing_result = Parser::new(src_bytes,
        // false).parse_all().map_err(|e| e.to_string());
        //
        // let _execution_result = match parsing_result {
        //   Ok(statements) => {
        //     println!("{}", statements);
        //     match statements.run(ctx) {
        //       Ok(v) => v,
        //       Err(e) => {
        //         dbg!(&e);
        //         panic!()
        //       }
        //     }
        //   }
        //   Err(e) => {
        //     info!("{:?}", &e);
        //     ctx.throw_syntax_error(e);
        //     panic!()
        //   }
        // };

        // let res = match ctx.eval(&source) {
        //   Ok(v) => v,
        //   Err(e) => {
        //     // dbg!(&e);
        //     info!("{:?}", e.get_type());
        //     info!("{:?}", e.to_object(ctx).unwrap().borrow().keys());
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

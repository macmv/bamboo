mod plugin;
mod types;

pub use plugin::Plugin;

use crate::{block, player::Player, world::WorldManager};
use common::math::Pos;
use std::{
  fmt, fs,
  sync::{Arc, Mutex},
};
use sugarlang::{define_ty, runtime::Var, Sugarlang};

#[derive(Debug)]
pub enum Event {
  Init,
  OnBlockPlace(Arc<Player>, Pos, block::Kind),
}

/// A struct that manages all Sugarlang plugins. This will handle re-loading all
/// the source files on `/reload`, and will also send events to all the plugins
/// when needed.
pub struct PluginManager {
  plugins: Mutex<Vec<Plugin>>,
}

#[derive(Clone)]
pub struct Sugarcane {
  plugin: String,
  wm:     Arc<WorldManager>,
}

impl Sugarcane {
  pub fn new(plugin: String, wm: Arc<WorldManager>) -> Self {
    Sugarcane { plugin, wm }
  }
}

#[define_ty(path = "sugarcane::Sugarcane")]
impl Sugarcane {
  pub fn info(&self, args: Variadic<Var>) {
    let mut msg = String::new();
    let mut iter = args.iter();
    if let Some(a) = iter.next() {
      msg += &format!("{}", a);
    }
    for a in iter {
      msg += &format!(" {}", a);
    }
    info!("plugin `{}`: {}", self.plugin, msg);
  }
}

impl fmt::Debug for Sugarcane {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Sugarcane {{}}")
  }
}

// impl Sugarcane {
//   fn new(wm: Arc<WorldManager>) -> Self {
//     Sugarcane { _wm: wm }
//   }
//   fn info(_this: &Value, args: &[Value], ctx: &mut Context) -> Result<Value>
// {     info!("{}", args.get(0).cloned().unwrap_or_default().to_string(ctx)?);
//     Ok(Value::Null)
//   }
//   fn add_plugin(this: &Value, args: &[Value], ctx: &mut Context) ->
// Result<Value> {     if let ObjectData::NativeObject(s) =
// &this.to_object(ctx)?.borrow().data {       info!("got self: {:?}", s);
//     } else {
//       error!("gaming?");
//     }
//     info!("{}", args[0].to_string(ctx).unwrap());
//     Ok(Value::Null)
//   }
// }

// impl Sugarcane {
//   fn constructor(_this: &Value, _args: &[Value], _ctx: &mut Context) ->
// Result<Self> {     Err(Value::String("cannot construct Sugarcane from
// JS".into()))   }
//
//   fn init(class: &mut ClassBuilder) -> Result<()> {
//     class.static_method("info", 0, Self::info);
//     class.method("add_plugin", 0, Self::add_plugin);
//
//     Ok(())
//   }
// }

impl PluginManager {
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  pub fn new() -> Self {
    PluginManager { plugins: Mutex::new(vec![]) }
  }

  pub fn add_builtins(sl: &mut Sugarlang) {
    sl.add_builtin_ty::<Sugarcane>();
    sl.add_builtin_ty::<types::SlPlayer>();
    sl.add_builtin_ty::<types::SlPos>();
    sl.add_builtin_ty::<types::SlBlockKind>();
  }

  /// Returns true if plugins should print error messages with colors.
  pub fn use_color(&self) -> bool {
    true
  }

  pub async fn run(&self, wm: Arc<WorldManager>) {
    self.load(wm);
    // let mut ctx = Context::new();
    // ctx.register_global_class::<Sugarcane>().unwrap();
    // let _o = ctx.construct_object();
    // ctx.register_global_property(
    //   "sc",
    //   Object::native_object(Box::new(Sugarcane::new(wm.clone()))),
    //   Attribute::all(),
    // );
    // self.load(&mut ctx, wm);
    //
    // let rx = self.rx.lock().unwrap();
    // self.handle_event(Event::Init);
    // loop {
    //   if let Ok(e) = rx.recv() {
    //     self.handle_event(e);
    //   }
    // }
  }

  fn handle_event(&self, _e: Event) {}

  /// Loads all plugins from disk. Call this to reload all plugins.
  fn load(&self, wm: Arc<WorldManager>) {
    let mut plugins = self.plugins.lock().unwrap();
    plugins.clear();
    for f in fs::read_dir("plugins").unwrap() {
      let f = f.unwrap();
      let m = fs::metadata(f.path()).unwrap();
      if m.is_file() {
        let path = f.path();
        info!("found plugin at {}", path.to_str().unwrap());
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        // let mut sl = Sugarlang::new();
        // sl.add_builtin_ty::<Sugarcane>();
        // sl.exec_statement("sugarcane::Sugarcane::init()");

        let mut p = Plugin::new(name, wm.clone());
        p.load_from_file(&path, self);

        p.call_init();

        plugins.push(p);
      }
    }
  }

  pub fn on_block_place(&self, player: Arc<Player>, pos: Pos, kind: block::Kind) {
    for p in self.plugins.lock().unwrap().iter() {
      p.call_on_block_place(player.clone(), pos, kind);
    }
  }
}

mod types;

use super::{PluginImpl, PluginManager, ServerMessage, Sugarcane};
use crate::{block, player::Player, world::WorldManager};
use sc_common::{math::Pos, net::sb::ClickWindow};
use std::{fs, path::Path, sync::Arc};
use sugarlang::{
  runtime::{LockedEnv, Path as SlPath, Path as TyPath, Var},
  SlError, Sugarlang,
};

/// A wrapper struct for a Panda plugin. This is used to execute Panda code
/// whenever an event happens.
pub struct PandaPlugin {
  name: String,
  sl:   Option<Sugarlang>,
  sc:   Sugarcane,
}

impl PandaPlugin {
  //   /// Creates a new plugin. The name should be the name of the module (for
  //   /// debugging) and the Module should be the ruby module for this plugin.
  pub fn new(idx: usize, name: String, wm: Arc<WorldManager>) -> Self {
    PandaPlugin { sc: Sugarcane::new(idx, name.clone(), wm), name, sl: None }
  }

  pub fn name(&self) -> &String { &self.name }

  /// This replaces the plugins envrionment with a new one, and then parses the
  /// given file as a sugarlang source file.
  pub fn load_from_file(&mut self, path: &Path, manager: &PluginManager) {
    self.sl = None;
    let mut sl = Sugarlang::new();
    sl.set_color(manager.use_color());
    self.add_builtins(&mut sl);
    match fs::read_to_string(path) {
      Ok(src) => {
        match sl.parse_file(&SlPath::new(vec![self.name.clone(), "main".into()]), path, src) {
          Ok(_) => {
            self.sl = Some(sl);
          }
          Err(err) => {
            self.sl = Some(sl);
            self.print_err(err);
            self.sl = None;
          }
        }
      }
      Err(err) => {
        warn!("{}", err);
      }
    }
  }

  /// This replaces the plugin envrionment with a new one, and then parses all
  /// of the files ending in `.sug` in the given directory.
  pub fn load_from_dir(&mut self, dir: &Path, manager: &PluginManager) {
    self.sl = None;
    let mut sl = Sugarlang::new();
    sl.set_color(manager.use_color());
    self.add_builtins(&mut sl);
    match sl.parse_dir(dir, &SlPath::new(vec![self.name.clone()])) {
      Ok(_) => {
        self.sl = Some(sl);
      }
      Err(err) => {
        self.sl = Some(sl);
        self.print_err(err);
        self.sl = None;
      }
    }
  }

  pub fn lock_env(&mut self) -> LockedEnv {
    let sl = self.sl.as_mut().unwrap();
    let (env, files) = sl.env_files();
    env.lock(files)
  }
  /// Returns a cloned Sugarcane struct. This should be used to call Sugarlang
  /// functions.
  pub fn sc(&self) -> Sugarcane { self.sc.clone() }

  fn path(&self, name: &str) -> SlPath {
    SlPath::new(vec![self.name.clone(), "main".into(), name.into()])
  }

  pub fn call_init(&self) { self.call(self.path("init"), vec![]); }
  pub fn call_on_block_place(&self, player: Arc<Player>, pos: Pos, kind: block::Kind) {
    self.call(
      self.path("on_block_place"),
      vec![
        types::player::SlPlayer::from(player).into(),
        types::util::SlPos::from(pos).into(),
        types::block::SlBlockKind::from(kind).into(),
      ],
    );
  }
  pub fn call_on_click_window(&self, player: Arc<Player>, slot: i32, mode: ClickWindow) -> bool {
    match self.call(
      self.path("on_click_window"),
      vec![
        types::player::SlPlayer::from(player).into(),
        slot.into(),
        types::item::SlClickWindow::from(mode).into(),
      ],
    ) {
      Var::Bool(v) => v,
      _ => true,
    }
  }

  pub fn call(&self, path: TyPath, args: Vec<Var>) -> Var {
    match &self.sl {
      Some(sl) => {
        if !sl.has_func(&path) {
          return Var::None;
        }
        match sl.call_args(&path, args) {
          Ok(v) => v,
          Err(e) => {
            self.print_err(e);
            Var::None
          }
        }
      }
      None => Var::None,
    }
  }

  pub fn print_err<E: SlError>(&self, err: E) {
    match &self.sl {
      Some(sl) => warn!("error in plugin `{}`:\n{}", self.name, sl.gen_err(err)),
      None => panic!("cannot print error without a sugarlang envrionment present!"),
    }
  }
}

impl PluginImpl for PandaPlugin {
  fn call(&self, ev: ServerMessage) -> Result<(), ()> { Ok(()) }
  fn panda(&mut self) -> Option<&mut PandaPlugin> { Some(self) }
}
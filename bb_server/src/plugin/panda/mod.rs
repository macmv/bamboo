use super::{
  types, Bamboo, GlobalServerEvent, PluginImpl, PluginManager, ServerEvent, ServerMessage,
};
use crate::{block, player::Player, world::WorldManager};
use bb_common::{math::Pos, net::sb::ClickWindow};
use panda::{
  runtime::{LockedEnv, Path as PdPath, Path as TyPath, Var},
  Panda, PdError,
};
use std::{fs, path::Path, sync::Arc};

/// A wrapper struct for a Panda plugin. This is used to execute Panda code
/// whenever an event happens.
pub struct PandaPlugin {
  name: String,
  sl:   Option<Panda>,
  bb:   Bamboo,
}

impl PandaPlugin {
  /// Creates a new plugin. The name should be the name of the plugin (for
  /// logs).
  pub fn new(idx: usize, name: String, wm: Arc<WorldManager>) -> Self {
    PandaPlugin { bb: Bamboo::new(idx, wm), name, sl: None }
  }

  pub fn name(&self) -> &String { &self.name }

  /// This replaces the plugins envrionment with a new one, and then parses the
  /// given file as a panda source file.
  pub fn load_from_file(&mut self, path: &Path, manager: &PluginManager) {
    self.sl = None;
    let mut sl = Panda::new();
    sl.set_color(manager.use_color());
    self.add_builtins(&mut sl);
    match fs::read_to_string(path) {
      Ok(src) => {
        match sl.parse_file(&PdPath::new(vec![self.name.clone(), "main".into()]), path, src) {
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
    let mut sl = Panda::new();
    sl.set_color(manager.use_color());
    self.add_builtins(&mut sl);
    match sl.parse_dir(dir, &PdPath::new(vec![self.name.clone()])) {
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
  /// Returns a cloned Bamboo struct. This should be used to call Panda
  /// functions.
  pub fn bb(&self) -> Bamboo { self.bb.clone() }

  fn path(&self, name: &str) -> PdPath {
    PdPath::new(vec![self.name.clone(), "main".into(), name.into()])
  }

  pub fn call_init(&self) { self.call(self.path("init"), vec![]); }
  pub fn call_on_block_place(&self, player: Arc<Player>, pos: Pos, ty: block::Type) -> bool {
    if let Var::Bool(allow) = self.call(
      self.path("on_block_place"),
      vec![
        types::player::PdPlayer::from(player).into(),
        types::util::PdPos::from(pos).into(),
        types::block::PdBlockKind::from(ty.kind()).into(),
      ],
    ) {
      allow
    } else {
      true
    }
  }
  pub fn call_on_block_break(&self, player: Arc<Player>, pos: Pos, ty: block::Type) -> bool {
    if let Var::Bool(allow) = self.call(
      self.path("on_block_break"),
      vec![
        types::player::PdPlayer::from(player).into(),
        types::util::PdPos::from(pos).into(),
        types::block::PdBlockKind::from(ty.kind()).into(),
      ],
    ) {
      allow
    } else {
      true
    }
  }
  pub fn call_on_click_window(&self, player: Arc<Player>, slot: i32, mode: ClickWindow) -> bool {
    match self.call(
      self.path("on_click_window"),
      vec![
        types::player::PdPlayer::from(player).into(),
        slot.into(),
        types::item::PdClickWindow::from(mode).into(),
      ],
    ) {
      Var::Bool(v) => v,
      _ => true,
    }
  }
  pub fn call_on_chat_message(&self, player: Arc<Player>, text: String) {
    self.call(
      self.path("on_chat_message"),
      vec![types::player::PdPlayer::from(player).into(), text.into()],
    );
  }
  pub fn call_on_player_join(&self, player: Arc<Player>) {
    self.call(self.path("on_player_join"), vec![types::player::PdPlayer::from(player).into()]);
  }
  pub fn call_on_player_leave(&self, player: Arc<Player>) {
    self.call(self.path("on_player_leave"), vec![types::player::PdPlayer::from(player).into()]);
  }
  pub fn call_on_tick(&self) { self.call(self.path("on_tick"), vec![]); }

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

  pub fn print_err<E: PdError>(&self, err: E) {
    match &self.sl {
      Some(sl) => warn!("error in plugin `{}`:\n{}", self.name, sl.gen_err(err)),
      None => panic!("cannot print error without a panda environment present!"),
    }
  }
}

impl PluginImpl for PandaPlugin {
  fn call(&self, ev: ServerMessage) -> Result<bool, ()> {
    match ev {
      ServerMessage::Event { player, event } => match event {
        ServerEvent::BlockPlace { pos, block } => {
          return Ok(self.call_on_block_place(player, pos, block))
        }
        ServerEvent::BlockBreak { pos, block } => {
          return Ok(self.call_on_block_break(player, pos, block))
        }
        ServerEvent::Chat { text } => self.call_on_chat_message(player, text),
        ServerEvent::ClickWindow { slot, mode } => {
          return Ok(self.call_on_click_window(player, slot, mode))
        }
        ServerEvent::PlayerJoin {} => self.call_on_player_join(player),
        ServerEvent::PlayerLeave {} => self.call_on_player_leave(player),
      },
      ServerMessage::GlobalEvent { event } => match event {
        GlobalServerEvent::Tick => self.call_on_tick(),
      },
      _ => {}
    }
    Ok(true)
  }
  fn panda(&mut self) -> Option<&mut PandaPlugin> { Some(self) }
}

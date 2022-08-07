use super::{
  types,
  types::{player::PPlayer, Callback as BCallback},
  Bamboo, CallError, GlobalServerEvent, PluginImpl, PluginManager, PluginReply, ServerEvent,
  ServerRequest,
};
use crate::{block, player::Player, world::WorldManager};
use bb_common::{math::Pos, net::sb::ClickWindow};
use panda::{
  parse::Path as PdPath,
  runtime::{Callback, LockedEnv, Var},
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

impl BCallback for Callback {
  fn call_panda(
    &self,
    env: &mut panda::runtime::LockedEnv<'_>,
    args: Vec<panda::runtime::Var>,
  ) -> panda::runtime::Result<()> {
    self.call(env, args)?;
    Ok(())
  }
  fn box_clone(&self) -> Box<dyn BCallback> { Box::new(self.clone()) }
}

impl PandaPlugin {
  /// Creates a new plugin. The name should be the name of the plugin (for
  /// logs).
  pub fn new(idx: usize, name: String, wm: Arc<WorldManager>) -> Self {
    PandaPlugin { bb: Bamboo::new(idx, wm), name, sl: None }
  }

  pub fn name(&self) -> &String { &self.name }

  /// This replaces the plugins environment with a new one, and then parses the
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

  /// This replaces the plugin environment with a new one, and then parses all
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

  pub fn call_init(&self) { self.call("init", vec![]); }
  pub fn call_on_block_place(&self, player: Arc<Player>, pos: Pos, ty: block::Type) -> bool {
    self.call(
      "block_place",
      vec![
        types::player::PPlayer::from(player).into(),
        types::util::PPos::from(pos).into(),
        types::block::PBlockKind::from(ty.kind()).into(),
      ],
    );
    true
  }
  pub fn call_on_block_break(&self, player: Arc<Player>, pos: Pos, ty: block::Type) -> bool {
    self.call(
      "block_break",
      vec![
        types::player::PPlayer::from(player).into(),
        types::util::PPos::from(pos).into(),
        types::block::PBlockKind::from(ty.kind()).into(),
      ],
    );
    true
  }
  pub fn call_on_click_window(&self, player: Arc<Player>, slot: i32, mode: ClickWindow) -> bool {
    self.call(
      "click_window",
      vec![
        types::player::PPlayer::from(player).into(),
        slot.into(),
        types::item::PClickWindow::from(mode).into(),
      ],
    );
    true
  }
  pub fn call_on_chat_message(&self, player: Arc<Player>, text: String) {
    self.call("chat_message", vec![types::player::PPlayer::from(player).into(), text.into()]);
  }
  pub fn call_on_player_join(&self, player: Arc<Player>) {
    self.call("player_join", vec![types::player::PPlayer::from(player).into()]);
  }
  pub fn call_on_player_leave(&self, player: Arc<Player>) {
    self.call("player_leave", vec![types::player::PPlayer::from(player).into()]);
  }
  pub fn call_on_tick(&self) { self.call("tick", vec![]); }

  pub fn call(&self, name: &str, args: Vec<Var>) {
    match &self.sl {
      Some(sl) => match sl.run_callback(name, args) {
        Ok(v) => v,
        Err(e) => self.print_err(e),
      },
      None => {}
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
  fn call(&self, player: Arc<Player>, ev: ServerEvent) -> Result<(), CallError> {
    match ev {
      ServerEvent::Chat { text } => self.call_on_chat_message(player, text),
      ServerEvent::PlayerJoin {} => self.call_on_player_join(player),
      ServerEvent::PlayerLeave {} => self.call_on_player_leave(player),
    }
    Ok(())
  }
  fn call_global(&self, ev: GlobalServerEvent) -> Result<(), CallError> {
    match ev {
      GlobalServerEvent::Tick => self.call_on_tick(),
      _ => todo!("global event {ev:?}"),
    }
    Ok(())
  }
  fn req(&self, player: Arc<Player>, request: ServerRequest) -> Result<PluginReply, CallError> {
    Ok(PluginReply::Cancel {
      allow: match request {
        ServerRequest::BlockPlace { pos, block } => {
          self.call_on_block_place(player, pos, block.ty())
        }
        ServerRequest::BlockBreak { pos, block } => {
          self.call_on_block_break(player, pos, block.ty())
        }
        ServerRequest::ClickWindow { slot, mode } => self.call_on_click_window(player, slot, mode),
      },
    })
  }
  fn panda(&mut self) -> Option<&mut PandaPlugin> { Some(self) }
}

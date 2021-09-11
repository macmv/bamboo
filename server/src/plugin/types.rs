use super::{PluginManager, Sugarcane};
use crate::{
  block,
  command::{Command, Parser},
  player::Player,
};
use common::math::Pos;
use std::sync::Arc;
use sugarlang::{
  define_ty,
  runtime::{Callback, Var},
  Sugarlang,
};

macro_rules! wrap {
  ( $ty:ty, $new_ty:ident ) => {
    #[derive(Clone, Debug)]
    pub struct $new_ty {
      inner: $ty,
    }

    impl From<$ty> for $new_ty {
      fn from(inner: $ty) -> $new_ty {
        $new_ty { inner }
      }
    }
  };
}

wrap!(Arc<Player>, SlPlayer);
wrap!(Pos, SlPos);
wrap!(block::Kind, SlBlockKind);
wrap!(Command, SlCommand);

#[define_ty(path = "sugarcane::Sugarcane")]
impl Sugarcane {
  pub fn info(&self, args: Variadic<&Var>) {
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

  pub fn add_command(&self, command: &SlCommand, callback: Callback) {
    let wm = self.wm.clone();
    let command = command.inner.clone();
    tokio::spawn(async move {
      wm.default_world()
        .await
        .get_commands()
        .add(command, |_, _| async move { info!("custom command") })
        .await;
    });
  }
}

#[define_ty(path = "sugarcane::Player")]
impl SlPlayer {
  pub fn username(&self) -> String {
    self.inner.username().into()
  }
}

#[define_ty(path = "sugarcane::Pos")]
impl SlPos {
  pub fn x(&self) -> i32 {
    self.inner.x()
  }
}

#[define_ty(path = "sugarcane::BlockKind")]
impl SlBlockKind {
  pub fn to_s(&self) -> String {
    format!("{:?}", self.inner)
  }
}

#[define_ty(path = "sugarcane::Command")]
impl SlCommand {
  pub fn new(name: &str) -> Self {
    SlCommand { inner: Command::new(name) }
  }
  pub fn add_arg_block_pos(&mut self, name: &str) {
    self.inner.add_arg(name, Parser::BlockPos);
  }
}

impl PluginManager {
  pub fn add_builtins(sl: &mut Sugarlang) {
    sl.add_builtin_ty::<Sugarcane>();
    sl.add_builtin_ty::<SlPlayer>();
    sl.add_builtin_ty::<SlPos>();
    sl.add_builtin_ty::<SlBlockKind>();
    sl.add_builtin_ty::<SlCommand>();
  }
}

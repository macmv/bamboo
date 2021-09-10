use super::{PluginManager, Sugarcane};
use crate::{block, command::Command, player::Player};
use common::math::Pos;
use std::sync::Arc;
use sugarlang::{define_ty, Sugarlang};

macro_rules! wrap {
  ( $ty:ty, $new_ty:ident ) => {
    #[derive(Clone, Debug)]
    pub struct $new_ty {
      inner: $ty,
    }

    impl $new_ty {
      pub fn new(inner: $ty) -> Self {
        $new_ty { inner }
      }
    }
  };
}

wrap!(Arc<Player>, SlPlayer);
wrap!(Pos, SlPos);
wrap!(block::Kind, SlBlockKind);
wrap!(Command, SlCommand);

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
  pub fn add_arg(&self) -> String {
    format!("{:?}", self.inner)
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

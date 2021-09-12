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
  docs::{markdown, MarkdownSection},
  path,
  runtime::{Callback, Var, VarRef},
  Sugarlang,
};

macro_rules! add_from {
  ( $ty:ty, $new_ty:ident ) => {
    impl From<$ty> for $new_ty {
      fn from(inner: $ty) -> $new_ty {
        $new_ty { inner }
      }
    }
  };
}

macro_rules! wrap {
  ( $ty:ty, $new_ty:ident ) => {
    #[derive(Clone, Debug)]
    pub struct $new_ty {
      inner: $ty,
    }

    add_from!($ty, $new_ty);
  };

  ( $ty:ty, $new_ty:ident, $extra:ident: $extra_ty:ty ) => {
    #[derive(Clone, Debug)]
    pub struct $new_ty {
      inner:  $ty,
      $extra: $extra_ty,
    }
  };
}

wrap!(Arc<Player>, SlPlayer);
wrap!(Pos, SlPos);
wrap!(block::Kind, SlBlockKind);
wrap!(Command, SlCommand, callback: Callback);

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

  pub fn add_command(&self, command: &SlCommand) {
    let wm = self.wm.clone();
    let cb = command.callback.clone();
    let command = command.inner.clone();
    let idx = self.idx;
    tokio::spawn(async move {
      wm.default_world()
        .await
        .get_commands()
        .add(command, move |_, _| {
          let wm = wm.clone();
          let mut cb = cb.clone();
          async move {
            let world = wm.default_world().await;
            let mut lock = world.get_plugins().plugins.lock().unwrap();
            let plugin = &mut lock[idx];
            let sc = plugin.sc();
            cb.call(&mut plugin.lock_env(), vec![VarRef::Owned(sc.into())]).unwrap();
          }
        })
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

/// A block position. This stores an X, Y, and Z coordinates.
///
/// If you need a player position, use
#[define_ty(path = "sugarcane::BPos")]
impl SlPos {
  /// Returns the X position of this block position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.x() // returns 5
  /// ```
  pub fn x(&self) -> i32 {
    self.inner.x()
  }
  /// Returns the Y position of this block position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.y() // returns 6
  /// ```
  pub fn y(&self) -> i32 {
    self.inner.y()
  }
  /// Returns the Z position of this block position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.z() // returns 7
  /// ```
  pub fn z(&self) -> i32 {
    self.inner.z()
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
  pub fn new(name: &str, callback: Callback) -> Self {
    SlCommand { inner: Command::new(name), callback }
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

    let docs = sl.generate_docs(&[(
      path!(sugarcane),
      markdown!(
        /// The sugarcane API. This is how all sugarlang code can interact
        /// with the sugarcane minecraft server. To get started with
        /// a variable called
        ///
        /// # Examples
        ///
        /// ```
        /// Type::asdhglas()
        /// ```
      ),
    )]);
    docs.save("target/sl_docs");
  }
}

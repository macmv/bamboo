use super::{PluginManager, Sugarcane};
use crate::{
  block,
  command::{Command, Parser},
  player::Player,
};
use sc_common::{
  math::{FPos, Pos},
  util::{chat::Color, Chat},
};
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
wrap!(FPos, SlFPos);
wrap!(block::Kind, SlBlockKind);
wrap!(Command, SlCommand, callback: Callback);

/// This is a handle into the Sugarcane server. It allows you to modify the
/// world, add commands, lookup players, and more. It will be passed to every
/// callback, so you should not store this in a global (although you can if you
/// need to).
#[define_ty(path = "sugarcane::Sugarcane")]
impl Sugarcane {
  /// Prints out the given arguments as an information message.
  ///
  /// # Example
  ///
  /// ```
  /// sc.info("some information")
  /// sc.info(5, 6)
  /// sc.info(my_vars, other, info)
  /// ```
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

  /// Adds a command to the server.
  ///
  /// # Example
  ///
  /// ```
  /// fn main() {
  ///   c = Command::new("setblock", handle_setblock)
  ///   sc.add_command(c)
  /// }
  ///
  /// fn handle_setblock(sc, player, args) {
  ///   sc.info("ran setblock!")
  /// }
  /// ```
  pub fn add_command(&self, command: &SlCommand) {
    let wm = self.wm.clone();
    let cb = command.callback.clone();
    let command = command.inner.clone();
    let idx = self.idx;
    tokio::spawn(async move {
      wm.default_world()
        .await
        .get_commands()
        .add(command, move |_, player, args| {
          let wm = wm.clone();
          let mut cb = cb.clone();
          async move {
            let world = wm.default_world().await;
            // We need this awkward scoping setup to avoid borrowing errors, and to make
            // sure `lock` doesn't get sent between threads.
            let mut err = None;
            let mut has_err = false;
            {
              let mut lock = world.get_plugins().plugins.lock().unwrap();
              let plugin = &mut lock[idx];
              let sc = plugin.sc();
              if let Err(e) = cb.call(&mut plugin.lock_env(), vec![VarRef::Owned(sc.into())]) {
                err = Some(e);
                has_err = true;
              }
              if let Some(e) = err {
                plugin.print_err(e);
              }
            }
            if has_err {
              if let Some(p) = player {
                let mut out = Chat::new("");
                out.add("Error executing command: ").color(Color::Red);
                out.add(format!("`{}` encountered an internal error", args[0].lit()));
                p.send_message(&out).await;
              }
            }
          }
        })
        .await;
    });
  }
}

/// A Player. This struct is for online players. If anyone has disconnected,
/// this struct will still exist, but the functions will return outdated
/// information. There is currently no way to lookup an offline player.
#[define_ty(path = "sugarcane::Player")]
impl SlPlayer {
  /// Returns the username of the player. This will never change, as long as the
  /// user stays online.
  pub fn username(&self) -> String {
    self.inner.username().into()
  }
}

/// A block position. This stores X, Y, and Z coordinates.
///
/// If you need a player position, use `FPos` instead.
#[define_ty(path = "sugarcane::Pos")]
impl SlPos {
  /// Returns the X position of this block.
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
  /// Returns the Y position of this block.
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
  /// Returns the Z position of this block.
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

/// An entity position. This stores X, Y, and Z coordinates as floats.
///
/// If you need a block position, use `Pos` instead.
#[define_ty(path = "sugarcane::FPos")]
impl SlFPos {
  /// Returns the X position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.x() // returns 5.5
  /// ```
  pub fn x(&self) -> f64 {
    self.inner.x()
  }
  /// Returns the Y position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.y() // returns 6.0
  /// ```
  pub fn y(&self) -> f64 {
    self.inner.y()
  }
  /// Returns the Z position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.z() // returns 7.2
  /// ```
  pub fn z(&self) -> f64 {
    self.inner.z()
  }
}

/// A block kind. This is how you get/set blocks in the world.
#[define_ty(path = "sugarcane::BlockKind")]
impl SlBlockKind {
  pub fn to_s(&self) -> String {
    format!("{:?}", self.inner)
  }
}

/// A command. This is how to setup the arguments for a custom commands that
/// users can run.
#[define_ty(path = "sugarcane::Command")]
impl SlCommand {
  /// Creates a new command. The callback must be a function, which takes 3
  /// arguments. See the example for details.
  ///
  /// # Example
  ///
  /// ```
  /// fn main() {
  ///   c = Command::new("setblock", handle_setblock)
  /// }
  ///
  /// fn handle_setblock(sc, player, args) {
  ///   sc.info("ran setblock!")
  /// }
  /// ```
  pub fn new(name: &str, callback: Callback) -> Self {
    SlCommand { inner: Command::new(name), callback }
  }
  /// Adds a new block position argument to the command.
  ///
  /// This will be parsed as three numbers in a row. If you use a `~` before the
  /// block coordinates, they will be parsed as relative coordinates. So if you
  /// are standing at X: 50, then `~10` will be converted into X: 60.
  pub fn add_arg_block_pos(&mut self, name: &str) {
    self.inner.add_arg(name, Parser::BlockPos);
  }
}

impl PluginManager {
  pub fn add_builtins(sl: &mut Sugarlang) {
    sl.add_builtin_ty::<Sugarcane>();
    sl.add_builtin_ty::<SlPlayer>();
    sl.add_builtin_ty::<SlPos>();
    sl.add_builtin_ty::<SlFPos>();
    sl.add_builtin_ty::<SlBlockKind>();
    sl.add_builtin_ty::<SlCommand>();

    let docs = sl.generate_docs(&[(
      path!(sugarcane),
      markdown!(
        /// The sugarcane API. This is how all sugarlang code can interact
        /// with the sugarcane minecraft server. To get started with writing
        /// a plugin, create a directory called `plugins` next to the server.
        /// Inside that dirctory, create a file named something like `hello.sug`.
        /// In that file, put the following code:
        ///
        /// ```
        /// fn init(sc) {
        ///   sc.info("hello world")
        /// }
        /// ```
        ///
        /// The given variable (`sc`) is a `Sugarcane` builtin. It is how you
        /// interact with the entire server. You can lookup worlds and players,
        /// add commands, and more. To start doing more things with your plugin,
        /// check out the docs for the `Sugarcane` type.
      ),
    )]);
    docs.save("target/sl_docs");
  }
}

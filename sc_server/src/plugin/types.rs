use super::{PluginManager, Sugarcane};
use crate::{
  block,
  command::{Arg, Command, Parser},
  player::Player,
};
use sc_common::{
  math::{FPos, Pos},
  util::{chat::Color, Chat},
};
use std::sync::{Arc, Mutex};
use sugarlang::{
  define_ty,
  docs::{markdown, MarkdownSection},
  parse::token::Span,
  path,
  runtime::{Callback, RuntimeError, Var, VarData, VarRef},
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
wrap!(Arc<Mutex<Chat>>, SlChat);
wrap!(Arc<Mutex<Chat>>, SlChatSection, idx: usize);
wrap!(Pos, SlPos);
wrap!(FPos, SlFPos);
wrap!(block::Kind, SlBlockKind);
wrap!(Command, SlCommand, callback: Callback);
wrap!(Arg, SlArg);

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
    let wm2 = self.wm.clone();
    let cb = command.callback.clone();
    let command = command.inner.clone();
    let idx = self.idx;
    tokio::spawn(async move {
      wm.get_commands()
        .add(command, move |_, player, args| {
          let wm = wm2.clone();
          let mut cb = cb.clone();
          async move {
            // We need this awkward scoping setup to avoid borrowing errors, and to make
            // sure `lock` doesn't get sent between threads.
            let mut err = None;
            let mut has_err = false;
            {
              let mut lock = wm.get_plugins().plugins.lock().unwrap();
              let plugin = &mut lock[idx];
              let sc = plugin.sc();
              if let Err(e) = cb.call(
                &mut plugin.lock_env(),
                vec![
                  VarRef::Owned(sc.into()),
                  player
                    .as_ref()
                    .map(|p| VarRef::Owned(SlPlayer::from(p.clone()).into()))
                    .unwrap_or(VarRef::Owned(Var::None)),
                  VarRef::Owned(
                    args
                      .iter()
                      .map(|arg| SlArg::from(arg.clone()).into())
                      .collect::<Vec<Var>>()
                      .into(),
                  ),
                ],
              ) {
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

  /// Sends the given chat message to a player. This accepts exactly one
  /// argument, which can be any type. If it is a `SlChat`, then it will be
  /// formatted correctly. Anything else will show up with debug formatting.
  ///
  /// # Example
  ///
  /// ```
  /// // The text `Hello!` will show up the the user's chat box.
  /// p.send_message("Hello!")
  ///
  /// chat = Chat::new()
  /// chat.add("I").color("red")
  /// chat.add(" am").color("gold")
  /// chat.add(" colors!").color("yellow")
  /// // The text `I am colors!` will show up in the user's chat box, colored
  /// // in red, then gold, then yellow.
  /// p.send_message(chat)
  /// ```
  pub fn send_message(&self, msg: &Var) {
    let p = self.inner.clone();
    let out = match msg {
      Var::Builtin(_, data) => {
        let chat = data.as_any().downcast_ref::<SlChat>();
        if let Some(chat) = chat {
          chat.inner.lock().unwrap().clone()
        } else {
          Chat::new(msg.to_string())
        }
      }
      _ => Chat::new(msg.to_string()),
    };
    tokio::spawn(async move {
      p.send_message(&out).await;
    });
  }
}

/// A chat message. This is how you can send formatted chat message to players.
#[define_ty(path = "sugarcane::Chat")]
impl SlChat {
  /// Creates an empty chat message. This can have sections added using `add`.
  pub fn empty() -> SlChat {
    SlChat { inner: Arc::new(Mutex::new(Chat::empty())) }
  }
  /// Adds a new chat section. This will return the section that was just added,
  /// so that it can be modified.
  ///
  /// # Example
  ///
  /// ```
  /// chat = Chat::empty()
  ///
  /// chat.add("hello").color("red")
  /// //   ^^^^^^^^^^^^ ------------ This is a function on `ChatSection`, which changes it's color.
  /// //   |
  /// //    \ Adds the section "hello"
  /// ```
  pub fn add(&self, msg: &str) -> SlChatSection {
    let mut lock = self.inner.lock().unwrap();
    lock.add(msg);
    SlChatSection { inner: self.inner.clone(), idx: lock.sections_len() - 1 }
  }
}

/// A chat message section. This section knows which chat message it came from.
/// All of the functions on this section will modify the chat message this came
/// from.
#[define_ty(path = "sugarcane::ChatSection")]
impl SlChatSection {
  /// Sets the color of this chat section. Since Sugarlang does not support
  /// enums, the color is simply a string. An invalid color will result in an
  /// error.
  ///
  /// # Example
  ///
  /// ```
  /// chat = Chat::empty()
  ///
  /// // Adds a new section, with the color set to red.
  /// chat.add("hello").color("red")
  /// ```
  pub fn color(&self, color: &str) -> Result<(), RuntimeError> {
    let col = match color {
      "red" => Color::Red,
      "yellow" => Color::Yellow,
      "green" => Color::BrightGreen,
      _ => return Err(RuntimeError::custom(format!("invalid color `{}`", color), Span::default())),
    };
    self.inner.lock().unwrap().get_section(self.idx).unwrap().color(col);
    Ok(())
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
  pub fn new(name: &str, callback: Callback) -> SlCommand {
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

/// A command argument. This is how you read back the arguments that a user
/// passed to your command.
#[define_ty(path = "sugarcane::Arg")]
impl SlArg {
  /// If this argument is a literal, then this returns the value of that
  /// literal. Otherwise, this will return an error.
  pub fn lit(&self) -> String {
    self.inner.lit().to_string()
  }
}

impl PluginManager {
  pub fn add_builtins(sl: &mut Sugarlang) {
    sl.add_builtin_ty::<Sugarcane>();
    sl.add_builtin_ty::<SlPlayer>();
    sl.add_builtin_ty::<SlChat>();
    sl.add_builtin_ty::<SlChatSection>();
    sl.add_builtin_ty::<SlPos>();
    sl.add_builtin_ty::<SlFPos>();
    sl.add_builtin_ty::<SlBlockKind>();
    sl.add_builtin_ty::<SlCommand>();
    sl.add_builtin_ty::<SlArg>();

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

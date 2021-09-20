use super::{PluginManager, Sugarcane};
use sc_common::util::{chat::Color, Chat};
use sugarlang::{
  define_ty,
  docs::{markdown, MarkdownSection},
  parse::token::Span,
  path,
  runtime::{RuntimeError, Var, VarRef},
  Sugarlang,
};

pub mod block;
pub mod chat;
pub mod command;
pub mod player;
pub mod util;
pub mod world;

use command::SlCommand;

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
      pub(super) inner: $ty,
    }

    add_from!($ty, $new_ty);
  };

  ( $ty:ty, $new_ty:ident, $($extra:ident: $extra_ty:ty),* ) => {
    #[derive(Clone, Debug)]
    pub struct $new_ty {
      pub(super) inner:  $ty,
      $(
        pub(super) $extra: $extra_ty,
      )*
    }
  };
}

// Only want these to be public to local files.
use add_from;
use wrap;

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
  pub fn add_command(&self, command: &SlCommand) -> Result<(), RuntimeError> {
    let wm = self.wm.clone();
    let wm2 = self.wm.clone();
    let cb = match command.callback.clone() {
      Some(cb) => cb,
      None => {
        return Err(RuntimeError::custom(
          "cannot pass in child command! you must pass in a command created from `Command::new`",
          Span::default(),
        ))
      }
    };
    let command = command.inner.lock().unwrap().clone();
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
                    .map(|p| VarRef::Owned(player::SlPlayer::from(p.clone()).into()))
                    .unwrap_or(VarRef::Owned(Var::None)),
                  VarRef::Owned(
                    args
                      .iter()
                      .map(|arg| command::SlArg::from(arg.clone()).into())
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
    Ok(())
  }
}

impl PluginManager {
  pub fn add_builtins(sl: &mut Sugarlang) {
    sl.add_builtin_ty::<Sugarcane>();
    sl.add_builtin_ty::<util::SlPos>();
    sl.add_builtin_ty::<util::SlFPos>();
    sl.add_builtin_ty::<block::SlBlockKind>();
    sl.add_builtin_ty::<chat::SlChat>();
    sl.add_builtin_ty::<chat::SlChatSection>();
    sl.add_builtin_ty::<command::SlCommand>();
    sl.add_builtin_ty::<command::SlArg>();
    sl.add_builtin_ty::<player::SlPlayer>();
    sl.add_builtin_ty::<world::SlWorld>();

    let docs = sl.generate_docs(&[
      (
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
      ),
      (
        path!(sugarcane::block),
        markdown!(
          /// This module handles blocks. It includes types to manage block kinds,
          /// block types, and any other data for blocks.
        ),
      ),
      (
        path!(sugarcane::chat),
        markdown!(
          /// This module handles chat messages. It includes a chat message (`Chat`),
          /// and a chat section type (`ChatSection`).
        ),
      ),
      (
        path!(sugarcane::command),
        markdown!(
          /// This module handles commands. Since minecraft 1.13, commands have gotten
          /// somewhat complex. This module allows you to build a command from a tree
          /// of arguments, and parse those commands from the client.
          ///
          /// For 1.8 to 1.12 clients, auto complete is implemented on the server, so
          /// it seems similar to 1.13+. You should not need to worry about this. You
          /// should be focused on building a command that takes the right arguments,
          /// which will then be handled by the server and client for you.
          ///
          /// Here is an example of creating a simple command:
          ///
          /// ```
          /// fn init() {
          ///   // The first argument is the name of the command, and the second argument
          ///   // is the function to call when the command is executed by a client.
          ///   c = Command::new("fill", handle_fill)
          ///   // This will expect the word `rect` to be inserted into the command string.
          ///   c.add_lit("rect")
          ///     .add_arg_block_pos("min")
          ///     .add_arg_block_pos("max")
          ///   // This will expect the word `circle` to be inserted into the command string.
          ///   // Since we are working with the original `c` struct, this is added as a
          ///   // second path in the tree, next to the `rect` path.
          ///   c.add_lit("rect")
          ///     .add_arg_block_pos("center")
          ///     .add_arg_float("radius")
          /// }
          ///
          /// // `sc` is the Sugarcane struct, which gives you access to everything on
          /// // the server.
          /// // `player` is the player who sent this command.
          /// // `args` is the parsed arguments to your command. See `command::Arg` for
          /// // details on how to handle that.
          /// fn handle_fill(sc, player, args) {
          ///   player.send_message("You just ran /fill!")
          /// }
          /// ```
          ///
          /// This will build a command with a tree like so:
          ///
          /// ```
          ///      fill
          ///    /      \
          /// "rect"  "circle"
          ///   |        |
          /// `pos`    `pos`
          ///   |        |
          /// `pos`   `float`
          /// ```
        ),
      ),
      (
        path!(sugarcane::player),
        markdown!(
          /// This module handles the player. It only includes one struct, called `Player`.
          /// Players are very complex, so it makes sense not to bundle the player in with
          /// entities.
        ),
      ),
      (
        path!(sugarcane::util),
        markdown!(
          /// This module includes various utilities, such as block positions, chunk positions,
          /// byte buffers, things like UUID, and more.
        ),
      ),
    ]);
    docs.save("target/sl_docs");
  }
}

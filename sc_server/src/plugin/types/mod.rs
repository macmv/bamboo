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
    sl.add_builtin_ty::<chat::SlChat>();
    sl.add_builtin_ty::<chat::SlChatSection>();
    sl.add_builtin_ty::<command::SlCommand>();
    sl.add_builtin_ty::<command::SlArg>();
    sl.add_builtin_ty::<player::SlPlayer>();
    sl.add_builtin_ty::<block::SlBlockKind>();

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

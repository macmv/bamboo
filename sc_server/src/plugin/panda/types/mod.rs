use super::{super::PluginManager, PandaPlugin, Sugarcane};
use sc_common::util::{chat::Color, Chat};
use sugarlang::{
  define_ty,
  docs::{markdown, MarkdownSection},
  parse::token::Span,
  path,
  runtime::{RuntimeError, Var},
  Sugarlang,
};

pub mod block;
pub mod chat;
pub mod command;
pub mod item;
pub mod player;
pub mod util;
pub mod world;

use command::SlCommand;
use world::gen::SlBiome;

macro_rules! add_from {
  ( $ty:ty, $new_ty:ident ) => {
    impl From<$ty> for $new_ty {
      fn from(inner: $ty) -> $new_ty { $new_ty { inner } }
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
  /// Adds a command to the server.
  ///
  /// # Example
  ///
  /// ```
  /// fn main() {
  ///   c = Command::new("setblock", handle_setblock)
  ///   sugarcane::instance().add_command(c)
  /// }
  ///
  /// fn handle_setblock(player, args) {
  ///   sugarcane::info("ran setblock!")
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
          Span::call_site(),
        ))
      }
    };
    let command = command.inner.lock().unwrap().clone();
    let idx = self.idx;
    wm.commands().add(command, move |_, player, args| {
      let wm = wm2.clone();
      let mut cb = cb.clone();
      {
        // We need this awkward scoping setup to avoid borrowing errors, and to make
        // sure `lock` doesn't get sent between threads.
        let mut err = None;
        let mut has_err = false;
        {
          let mut lock = wm.plugins().plugins.lock();
          let plugin = &mut lock[idx];
          let panda = plugin.unwrap_panda();
          if let Err(e) = cb.call(
            &mut panda.lock_env(),
            vec![
              player.map(|p| player::SlPlayer::from(p.clone()).into()).unwrap_or(Var::None),
              args.iter().map(|arg| command::sl_from_arg(arg.clone())).collect::<Vec<Var>>().into(),
            ],
          ) {
            err = Some(e);
            has_err = true;
          }
          if let Some(e) = err {
            panda.print_err(e);
          }
        }
        if has_err {
          if let Some(p) = player {
            let mut out = Chat::new("");
            out.add("Error executing command: ").color(Color::Red);
            out.add(format!("`{}` encountered an internal error", args[0].lit()));
            p.send_message(&out);
          }
        }
      }
    });
    Ok(())
  }
  /// Adds a new biome to the server. This works ontop of a terrain generator.
  /// By default, each of the biomes are chosen at random, for various regions
  /// of the map. Then, the biome generation takes place in each of these
  /// regions. The behavior of an individual biome can be overriden using this
  /// function.
  ///
  /// # Example
  ///
  /// ```
  /// fn main(sc) {
  ///   biome = Biome::new("desert")
  ///
  ///   sc.add_biome(biome)
  /// }
  /// ```
  ///
  /// See the `Biome` docs for more.
  pub fn add_biome(&self, _biome: &SlBiome) -> Result<(), RuntimeError> { Ok(()) }

  /// Locks the internal data. If the internal data is already locked, this will
  /// continue trying to lock that data.
  pub fn lock(&self) -> Var {
    loop {
      match self.data.lock().take() {
        Some(v) => return v.into(),
        None => std::thread::yield_now(),
      }
    }
  }

  /// Locks the internal data, stores the given value, and then unlocks it. Note
  /// that this should be avoided if you have multiple threads modifying
  /// `data`. This can cause a race condition if you are not careful. If you
  /// need to hold onto a lock, then call `Sugarcane::lock` instead.
  pub fn store(&self, data: Var) -> Result<(), RuntimeError> {
    let mut lock = self.data.lock();
    if lock.is_none() {
      return Err(RuntimeError::custom("data is already locked!", Span::call_site()));
    }
    lock.replace(data.into());
    Ok(())
  }

  /// Releases the lock on the internal data. This will move the given data back
  /// into the lock. Note that if the lock is not held, this will return an
  /// error.
  pub fn unlock(&self, data: Var) -> Result<(), RuntimeError> {
    let prev = self.data.lock().replace(data.into());
    if let Some(prev) = prev {
      Err(RuntimeError::custom(
        format!("was not locked! existing data: {prev:?}"),
        Span::call_site(),
      ))
    } else {
      Ok(())
    }
  }
}

fn format(args: &[Var]) -> String {
  let mut msg = String::new();
  let mut iter = args.iter();
  if let Some(a) = iter.next() {
    msg += &format!("{}", a);
  }
  for a in iter {
    msg += &format!(" {}", a);
  }
  msg
}

impl PandaPlugin {
  pub fn add_builtins(&self, sl: &mut Sugarlang) {
    let sc = self.sc();
    sl.add_builtin_fn(path!(sugarcane::instance), false, move |_env, _slf, args, pos| {
      RuntimeError::check_arg_len(&args, 0, pos)?;
      Ok(sc.clone().into())
    });
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(sugarcane::trace), false, move |_env, _slf, args, _pos| {
        trace!("plugin `{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(sugarcane::debug), false, move |_env, _slf, args, _pos| {
        debug!("plugin `{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(sugarcane::info), false, move |_env, _slf, args, _pos| {
        info!("plugin `{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(sugarcane::warn), false, move |_env, _slf, args, _pos| {
        warn!("plugin `{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(sugarcane::error), false, move |_env, _slf, args, _pos| {
        error!("plugin `{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    sl.add_builtin_ty::<Sugarcane>();
    sl.add_builtin_ty::<util::SlPos>();
    sl.add_builtin_ty::<util::SlFPos>();
    sl.add_builtin_ty::<block::SlBlockKind>();
    sl.add_builtin_ty::<chat::SlChat>();
    sl.add_builtin_ty::<chat::SlChatSection>();
    sl.add_builtin_ty::<item::SlClickWindow>();
    sl.add_builtin_ty::<item::SlInventory>();
    sl.add_builtin_ty::<item::SlStack>();
    sl.add_builtin_ty::<item::SlUI>();
    sl.add_builtin_ty::<command::SlCommand>();
    sl.add_builtin_ty::<player::SlPlayer>();
    sl.add_builtin_ty::<world::SlWorld>();
    sl.add_builtin_ty::<world::gen::SlBiome>();
  }

  pub fn generate_docs(&self, sl: &Sugarlang) {
    let docs = sl.generate_docs(
      &[
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
            /// fn init() {
            ///   sugarcane::info("Hello world")
            /// }
            /// ```
            ///
            /// To start doing more things with your plugin, check out the docs
            /// for the `Sugarcane` type. You can access an instance of it like so:
            ///
            /// ```
            /// fn init() {
            ///   sc = sugarcane::instance()
            ///   sc.broadcast("This will show up in chat for everyone!")
            /// }
            /// ```
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
            /// // `args` is the parsed arguments to your command. Each item in this array
            /// // is typed according to the arguments. So, the first item will be a string
            /// // with the text "fill", the second will be a string will the text "rect"
            /// // or "circle", the third item will be a block position, and the last item will
            /// // be another block position or a float.
            ///
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
            ///
            /// This will parse all of these commands:
            ///
            /// ```
            /// /fill rect -3 -2 -1 4 5 6
            /// /fill circle ~ ~ ~ 3.5
            /// /fill rect ~-10 ~-10 ~-10 ~10 ~10 ~10
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
        (
          path!(sugarcane::world),
          markdown!(
            /// This module includes a World and WorldManager. The World allows you to spawn
            /// in entities, change blocks, mess with players, etc. The WorldManager allows
            /// you to teleport people between worlds, create new worlds, etc.
          ),
        ),
        (
          path!(sugarcane::world::gen),
          markdown!(
            /// This modules is for everything related to terrain generation. This is a complex
            /// module, simply becuase of how much there is to do.
          ),
        ),
      ],
      &[
        (
          path!(sugarcane::trace),
          markdown!(
            /// Prints out the given arguments as a `trace` log message.
          ),
        ),
        (
          path!(sugarcane::debug),
          markdown!(
            /// Prints out the given arguments as a `debug` log message.
          ),
        ),
        (
          path!(sugarcane::info),
          markdown!(
            /// Prints out the given arguments as an `info` log message.
            ///
            /// # Example
            ///
            /// ```
            /// sc.info("some information")
            /// sc.info(5, 6)
            /// sc.info(my_vars, other, info)
            /// ```
          ),
        ),
        (
          path!(sugarcane::warn),
          markdown!(
            /// Prints out the given arguments as a `warn` log message.
          ),
        ),
        (
          path!(sugarcane::error),
          markdown!(
            /// Prints out the given arguments as an `error` log message.
          ),
        ),
      ],
    );
    docs.save("target/sl_docs");
  }
}

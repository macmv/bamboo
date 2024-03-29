use super::{
  block::PBlockKind,
  item::PStack,
  util::{PChunkPos, PPos},
  Callback as BCallback,
};
use crate::{
  command::{Arg, Command, EntitySelector, Parser},
  player::Player,
  world::World,
};
use bb_server_macros::define_ty;
use panda::runtime::{Callback, Var};
use std::{
  fmt,
  sync::{Arc, Mutex, Weak},
};

impl Clone for PCommand {
  fn clone(&self) -> Self {
    PCommand {
      inner:    self.inner.clone(),
      callback: self.callback.as_ref().map(|c| c.box_clone()),
      idx:      self.idx.clone(),
    }
  }
}
impl fmt::Debug for PCommand {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("PCommand")
      .field("inner", &self.inner)
      .field("callback", &self.callback)
      .field("idx", &self.idx)
      .finish()
  }
}
impl fmt::Debug for PEntitySelector {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("PEntitySelector")
      .field("inner", &self.inner)
      .field("runner", &self.runner)
      .finish()
  }
}

impl PCommand {
  fn command<'a>(&self, inner: &'a mut Command) -> &'a mut Command {
    let mut c = inner;
    for idx in &self.idx {
      c = c.get_child(*idx).unwrap();
    }
    c
  }
}

pub fn sl_from_arg(
  arg: Arg,
  world: &Arc<World>,
  runner: Option<&Arc<Player>>,
) -> panda::runtime::Var {
  match arg {
    Arg::Literal(text) => text.into(),
    Arg::Bool(v) => v.into(),
    Arg::Double(v) => v.into(),
    Arg::Float(v) => v.into(),
    Arg::Int(v) => v.into(),
    Arg::String(v) => v.into(),
    Arg::Entity(inner) => {
      PEntitySelector { inner, runner: runner.map(Arc::downgrade), world: world.clone() }.into()
    }
    /*
    Arg::ScoreHolder(String),
    Arg::GameProfile(EntitySelector),
    */
    Arg::BlockPos(pos) => PPos::from(pos).into(),
    Arg::ColumnPos(pos) => PChunkPos::from(pos).into(),
    /*
    Arg::Vec3(f64, f64, f64),
    Arg::Vec2(f64, f64),
    */
    Arg::BlockState(kind, _props, _nbt) => PBlockKind::from(kind).into(),
    Arg::ItemStack(stack) => PStack::from(stack).into(),
    /*
    BlockPredicate(block::Kind),
    ItemPredicate(item::Type),
    Color(Color),
    Component(Chat),
    Message(String),
    Nbt(NBT),
    NbtPath(String),
    Objective(String),
    ObjectiveCriteria(String),
    Operation(String),
    Particle(String), // TODO: Particles
    Rotation(f32, f32),
    Angle(f32),
    ScoreboardSlot(String),
    Swizzle(f64, f64, f64),
    Team(String),
    /// A name for an inventory slot. Unclear on what is valid. Parsed as a string
    /// for now.
    ItemSlot(String),
    /// An identifier. Parsed as a string for now.
    ResourceLocation(String),
    /// A potion effect. Parsed as an identifier (things like `minecraft:foo`).
    MobEffect(String),
    /// A function. Also parsed as a string, because I do not know what this is.
    Function(String),
    /// Entity anchor. What even is this thing. Parsed as a string,
    EntityAnchor(String),
    Range {
      min: f64,
      max: f64,
    },
    IntRange {
      min: i32,
      max: i32,
    },
    FloatRange {
      min: f64,
      max: f64,
    },
    /// Represents a item enchantment. Parsed as a string.
    ItemEnchantment(String),
    /// Represents an entity summon. This will be a parsed entity type.
    EntitySummon(entity::Type),
    /// Represents a dimension. MORE STRINGS
    Dimension(String),
    Uuid(UUID),
    /// Different to nbt how?
    NbtTag(NBT),
    /// Once again, different to nbt how?
    NbtCompoundTag(NBT),
    Time(u64),

    /// A forge mod id
    Modid(String),
    /// A enum class to use for suggestion. Added by Minecraft Forge.
    Enum(String),
    */
    _ => todo!("command arg {:?}", arg),
  }
}

/// A command. This is how to setup the arguments for a custom commands that
/// users can run.
#[define_ty]
impl PCommand {
  info! {
    clone: false,
    debug: false,
    fields: {
      inner:    Arc<Mutex<Command>>,
      callback: Option<Box<dyn BCallback>>,
      idx:      Vec<usize>,
    },

    panda: {
      path: "bamboo::command::Command",
    },
    python: {
      class: "Command",
    },
  }
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
  /// fn handle_setblock(bb, player, args) {
  ///   bb.info("ran setblock!")
  /// }
  /// ```
  pub fn new(name: &str, callback: Callback) -> PCommand {
    PCommand {
      inner:    Arc::new(Mutex::new(Command::new(name))),
      callback: Some(Box::new(callback)),
      idx:      vec![],
    }
  }
  /// Adds a new block position argument to the command.
  ///
  /// This will be parsed as three numbers in a row. If you use a `~` before the
  /// block coordinates, they will be parsed as relative coordinates. So if you
  /// are standing at X: 50, then `~10` will be converted into X: 60.
  pub fn add_arg_block_pos(&mut self, name: &str) -> PCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_arg(name, Parser::BlockPos);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    PCommand { inner: self.inner.clone(), callback: None, idx }
  }
  /// Adds a new block kind argument to the command.
  ///
  /// This will be parsed as a single world, which will then be converted to a
  /// block kind. An invalid block kind will not read the callback, and will
  /// instead return an error to the user.
  pub fn add_arg_block_kind(&mut self, name: &str) -> PCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_arg(name, Parser::BlockState);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    PCommand { inner: self.inner.clone(), callback: None, idx }
  }
  /// Adds a new item kind argument to the command.
  ///
  /// This will be parsed as a single world, which will then be converted to an
  /// item kind. An invalid item kind will not read the callback, and will
  /// instead return an error to the user.
  pub fn add_arg_item_stack(&mut self, name: &str) -> PCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_arg(name, Parser::ItemStack);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    PCommand { inner: self.inner.clone(), callback: None, idx }
  }
  /// Adds a literal to the command.
  ///
  /// This is a special type of argument. It matches the exact text of the name.
  /// This should only be used if you want to expect a keyword.
  ///
  /// # Example
  ///
  /// ```
  /// c = Command::new("fill", handle_fill)
  /// c.add_arg_lit("rect")
  ///   .add_arg_block_pos("min")
  ///   .add_arg_block_pos("max")
  /// c.add_arg_lit("circle")
  ///   .add_arg_block_pos("center")
  ///   .add_arg_float("radius")
  /// ```
  ///
  /// This will parse the following commands:
  /// ```
  /// /fill rect ~ ~ ~ ~ ~ ~
  /// /fill rect 5 5 5 20 20 20
  ///
  /// /fill circle ~ ~ ~ 5
  /// /fill circle 6 7 8 20
  /// ```
  ///
  /// As you can see, this should only be used when you have a keyword you need
  /// the user to type in. See `add_arg_word` if you are expecting a single
  /// word.
  pub fn add_lit(&mut self, name: &str) -> PCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_lit(name);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    PCommand { inner: self.inner.clone(), callback: None, idx }
  }
}

/// An entity selector. This is the parsed version of either a player username,
/// `@a`, `@e`, `@p`, or another selection of entities.
#[define_ty]
impl PEntitySelector {
  info! {
    debug: false,
    fields: {
      inner:  EntitySelector,
      runner: Option<Weak<Player>>,
      world:  Arc<World>,
    },

    panda: {
      path: "bamboo::command::EntitySelector",
    },
    python: {
      class: "EntitySelector",
    },
  }

  /// Returns all the players selected by this selector.
  ///
  /// This iterates through the players, so calling this function is when the
  /// players list is captured. This means that storing the entity selector, and
  /// then calling `players` later will give different results from the first
  /// call.
  pub fn players(&self) -> Vec<Var> {
    self
      .inner
      .clone()
      .iter(
        &self.world.entities(),
        self.runner.as_ref().map(|weak| weak.upgrade()).unwrap_or(None).as_ref(),
      )
      .flat_map(|ent| ent.as_player().cloned().map(super::player::PPlayer::from).map(Into::into))
      .collect()
  }
}

use super::{
  util::{SlChunkPos, SlPos},
  wrap,
};
use crate::command::{Arg, Command, Parser};
use std::sync::{Arc, Mutex};
use sugarlang::{
  define_ty,
  runtime::{Callback, Var},
};

wrap!(Arc<Mutex<Command>>, SlCommand, callback: Option<Callback>, idx: Vec<usize>);

impl SlCommand {
  fn command<'a>(&self, inner: &'a mut Command) -> &'a mut Command {
    let mut c = inner;
    for idx in &self.idx {
      c = c.get_child(*idx).unwrap();
    }
    c
  }
}

pub fn sl_from_arg(arg: Arg) -> Var {
  match arg {
    Arg::Literal(text) => text.into(),
    Arg::Bool(v) => v.into(),
    Arg::Double(v) => v.into(),
    Arg::Float(v) => v.into(),
    Arg::Int(v) => v.into(),
    Arg::String(v) => v.into(),
    /*
    Arg::Entity(EntitySelector),
    Arg::ScoreHolder(String),
    Arg::GameProfile(EntitySelector),
    */
    Arg::BlockPos(pos) => SlPos::from(pos).into(),
    Arg::ColumnPos(pos) => SlChunkPos::from(pos).into(),
    /*
    Arg::Vec3(f64, f64, f64),
    Arg::Vec2(f64, f64),
    BlockState(block::Kind, HashMap<String, String>, Option<NBT>),
    BlockPredicate(block::Kind),
    ItemStack(item::Stack),
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
#[define_ty(path = "sugarcane::command::Command")]
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
    SlCommand {
      inner:    Arc::new(Mutex::new(Command::new(name))),
      callback: Some(callback),
      idx:      vec![],
    }
  }
  /// Adds a new block position argument to the command.
  ///
  /// This will be parsed as three numbers in a row. If you use a `~` before the
  /// block coordinates, they will be parsed as relative coordinates. So if you
  /// are standing at X: 50, then `~10` will be converted into X: 60.
  pub fn add_arg_block_pos(&mut self, name: &str) -> SlCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_arg(name, Parser::BlockPos);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    SlCommand { inner: self.inner.clone(), callback: None, idx }
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
  pub fn add_lit(&mut self, name: &str) -> SlCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_lit(name);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    SlCommand { inner: self.inner.clone(), callback: None, idx }
  }
}

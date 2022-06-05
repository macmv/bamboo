use crate::player::Player;
use parking_lot::{lock_api::RawMutex, Mutex};
use std::collections::HashMap;

pub struct Command {
  name:     String,
  ty:       NodeType,
  children: Vec<Command>,
  optional: bool,
}
#[derive(Debug, Clone)]
enum NodeType {
  Literal,
  Argument(String),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Arg {
  Lit(String),
}

impl Arg {
  pub fn new(carg: bb_ffi::CArg) -> Self {
    if let Some(s) = carg.into_literal() {
      Arg::Lit(s.into_string())
    } else {
      todo!()
    }
  }
  pub fn lit(&self) -> &str {
    match self {
      Self::Lit(s) => s.as_str(),
      _ => panic!("not a literal: {self:?}"),
    }
  }
}

static CALLBACKS: Mutex<Option<HashMap<String, Box<dyn Fn(Option<Player>, Vec<Arg>) + Send>>>> =
  Mutex::const_new(parking_lot::RawMutex::INIT, None);
pub fn add_command(cmd: &Command, cb: impl Fn(Option<Player>, Vec<Arg>) + Send + 'static) {
  {
    let mut cbs = CALLBACKS.lock();
    if cbs.is_none() {
      *cbs = Some(HashMap::new());
    }
    cbs.as_mut().unwrap().insert(cmd.name.clone(), Box::new(cb));
  }
  unsafe {
    let ffi = cmd.to_ffi();
    bb_ffi::bb_add_command(&ffi);
  }
}

impl Command {
  pub fn new(name: impl Into<String>) -> Self {
    Command {
      name:     name.into(),
      ty:       NodeType::Literal,
      children: vec![],
      optional: false,
    }
  }
  pub fn add_arg(&mut self, name: impl Into<String>, parser: impl Into<String>) -> &mut Command {
    self.children.push(Command {
      name:     name.into(),
      ty:       NodeType::Argument(parser.into()),
      children: vec![],
      optional: false,
    });
    self.children.last_mut().unwrap()
  }
  pub fn add_lit(&mut self, name: impl Into<String>) -> &mut Command {
    self.children.push(Command {
      name:     name.into(),
      ty:       NodeType::Literal,
      children: vec![],
      optional: false,
    });
    self.children.last_mut().unwrap()
  }

  /// # Safety
  /// - `self` is essentially borrowed for the entire lifetime of the returned
  ///   command. This command points to data in `self` which cannot be changed.
  pub(crate) unsafe fn to_ffi(&self) -> bb_ffi::CCommand {
    bb_ffi::CCommand {
      name:      bb_ffi::CStr::new(self.name.clone()),
      node_type: match self.ty {
        NodeType::Literal => 0,
        NodeType::Argument(_) => 1,
      },
      parser:    match &self.ty {
        NodeType::Literal => bb_ffi::CStr::new(String::new()),
        NodeType::Argument(parser) => parser.to_ffi(),
      },
      optional:  bb_ffi::CBool::new(self.optional),
      children:  bb_ffi::CList::new(self.children.iter().map(|c| c.to_ffi()).collect()),
    }
  }
}

#[no_mangle]
extern "C" fn on_command(player: *mut bb_ffi::CUUID, args: *mut bb_ffi::CList<bb_ffi::CArg>) {
  unsafe {
    let player = if player.is_null() { None } else { Some(Box::from_raw(player)) };
    let args = Box::from_raw(args);
    let args: Vec<_> = args.into_vec().into_iter().map(|carg| Arg::new(carg)).collect();
    let name = args[0].lit();
    let cb = CALLBACKS.lock();
    if let Some(cb) = cb.as_ref() {
      cb[name](player.map(|id| Player::new(*id)), args);
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parser {
  // Simple types:
  /// True or false.
  Bool,
  /// A double, with optional min and max values.
  Double { min: Option<f64>, max: Option<f64> },
  /// A float, with optional min and max values.
  Float { min: Option<f32>, max: Option<f32> },
  /// An int, with optional min and max values.
  Int { min: Option<i32>, max: Option<i32> },
  /// A string. See [`StringType`] for details on how this is parsed.
  String(StringType),
  /// An entity. If `single` is set, then this can only match one entity (things
  /// like `@e` or `@a` are not allowed). If players is set, then only matching
  /// players (with either a username, `@p`, etc.) is allowed.
  Entity { single: bool, only_players: bool },
  /// A user that is on the current scoreboard. With the scoreboard system that
  /// bamboo has, this doesn't make that much sense.
  ScoreHolder { multiple: bool },

  /// Player, online or not. Can also use a selector.
  GameProfile,
  /// location, represented as 3 numbers (which must be integers)
  BlockPos,
  /// column location, represented as 3 numbers (which must be integers)
  ColumnPos,
  /// A location, represented as 3 numbers
  Vec3,
  /// A location, represented as 2 numbers
  Vec2,
  /// A block state, optionally including NBT and state information.
  BlockState,
  /// A block, or a block tag.
  BlockPredicate,
  /// An item, optionally including NBT.
  ItemStack,
  /// An item, or an item tag.
  ItemPredicate,
  /// Chat color. One of the names from Chat#Colors, or reset.
  Color,
  /// A JSON Chat component.
  Component,
  /// A regular message, potentially including selectors.
  Message,
  /// An NBT value, parsed using JSON-NBT rules.
  Nbt,
  /// A path within an NBT value, allowing for array and member accesses.
  NbtPath,
  /// A scoreboard objective.
  Objective,
  /// A single score criterion.
  ObjectiveCriteria,
  /// A scoreboard operator.
  Operation,
  /// A particle effect
  Particle,
  /// angle, represented as 2 floats
  Rotation,
  /// A single float
  Angle,
  /// Scoreboard display position slot. list, sidebar, belowName, etc
  ScoreboardSlot,
  /// A collection of up to 3 axes.
  Swizzle,
  /// The name of a team. Parsed as an unquoted string.
  Team,
  /// A name for an inventory slot.
  ItemSlot,
  /// An Identifier.
  ResourceLocation,
  /// A potion effect.
  MobEffect,
  /// A function.
  Function,
  /// entity anchor related to the facing argument
  EntityAnchor,
  /// A range of values with a min and a max.
  Range { decimals: bool },
  /// An integer range of values with a min and a max.
  IntRange,
  /// A floating-point range of values with a min and a max.
  FloatRange,
  /// Represents a item enchantment.
  ItemEnchantment,
  /// Represents an entity summon.
  EntitySummon,
  /// Represents a dimension.
  Dimension,
  /// Represents a UUID value.
  Uuid,
  /// Represents a partial nbt tag, usable in data modify command.
  NbtTag,
  /// Represents a full nbt tag.
  NbtCompoundTag,
  /// Represents a time duration.
  Time,

  // Forge only types:
  /// A forge mod id
  Modid,
  /// A enum class to use for suggestion. Added by Minecraft Forge.
  Enum,
}

impl Parser {
  pub(crate) fn to_ffi(&self) -> bb_ffi::CCommandParser {}
}

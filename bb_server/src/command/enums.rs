use crate::{
  block, entity,
  entity::EntityRef,
  item,
  player::Player,
  world::{EntitiesIter, EntitiesMapRef},
};
use bb_common::{
  math::{ChunkPos, Pos},
  nbt::NBT,
  util::{
    chat::{Chat, Color},
    UUID,
  },
};
use std::{collections::HashMap, sync::Arc};

/// A string parsing type. Used only in [`Parser::String`].
#[derive(Debug, Clone, PartialEq)]
pub enum StringType {
  /// Matches a single word.
  Word,
  /// Matches either a single word, or a phrase in double quotes. Quotes can be
  /// inserted in the string with `\"`.
  Quotable,
  /// Matches all remaining text in the command. Quotes are not interpreted.
  Greedy,
}

/// This is a command argument parser. All of the information for this comes
/// from [wiki.vg](https://wiki.vg/Command_Data). They have a great collection
/// of data for all of this stuff, and this entire server wouldn't be possible
/// without them.
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

/// An entity selector (things like `@a`, `@p`, or just a username).
#[derive(Debug, Clone, PartialEq)]
pub enum EntitySelector {
  /// A username
  Name(String),
  /// All entites, with the given restrictions
  Entities(HashMap<String, String>), //
  /// All players, with the given restrictions
  Players(HashMap<String, String>),
  /// The player who ran the command (@s)
  Runner,
  /// The player who is closest (@p)
  Closest(HashMap<String, String>),
  /// Random player (@r)
  Random(HashMap<String, String>),
}

/// This is the result of a parsed command. It contains all the values from
/// Parser, but also contains the data that each argument contains.
///
/// I do not know what a lot of these types do. Most of them seem pointless, so
/// I have not bothered to see what they do ingame. Send a PR if you know how
/// this should work.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
  /// A parsed literal.
  Literal(String),

  Bool(bool),
  Double(f64),
  Float(f32),
  Int(i32),
  String(String),
  Entity(EntitySelector),
  ScoreHolder(String),
  GameProfile(EntitySelector),
  BlockPos(Pos),
  ColumnPos(ChunkPos),
  Vec3(f64, f64, f64),
  Vec2(f64, f64),
  // A block kind, with state info, and optional nbt
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
}

macro_rules! unwrapper_copy {
  ($name:ident, $enum:ident, $ty:ty) => {
    pub fn $name(&self) -> $ty {
      match self {
        Arg::$enum(v) => *v,
        _ => panic!(concat!("arg is a {:?}, not a ", stringify!($name)), self),
      }
    }
  };
}

impl Arg {
  unwrapper_copy!(double, Double, f64);
  unwrapper_copy!(float, Float, f32);
  unwrapper_copy!(int, Int, i32);
  unwrapper_copy!(pos, BlockPos, Pos);
  pub fn lit(&self) -> &str {
    match self {
      Arg::Literal(v) => v,
      _ => panic!("arg is a {:?}, not a literal", self),
    }
  }
  pub fn block(&self) -> block::Kind {
    match self {
      Arg::BlockState(kind, _, _) => *kind,
      _ => panic!("arg is a {:?}, not a block", self),
    }
  }
  pub fn str(&self) -> &str {
    match self {
      Arg::String(v) => v,
      _ => panic!("arg is a {:?}, not a string", self),
    }
  }
  pub fn entity(&self) -> EntitySelector {
    match self {
      Arg::Entity(v) => v.clone(),
      _ => panic!("arg is a {:?}, not an entity", self),
    }
  }
  pub fn entity_summon(&self) -> entity::Type {
    match self {
      Arg::EntitySummon(v) => *v,
      _ => panic!("arg is a {:?}, not an entity summon", self),
    }
  }
}

pub enum EntityIter<'a> {
  /// A player
  Player(Option<Arc<Player>>),
  /// All entites, with the given restrictions
  Entities { props: HashMap<String, String>, iter: EntitiesIter<'a> },
  /// All players, with the given restrictions
  Players { props: HashMap<String, String>, iter: EntitiesIter<'a> },
}

impl EntitySelector {
  pub fn iter<'a>(
    self,
    entities: &'a EntitiesMapRef<'a>,
    runner: Option<&Arc<Player>>,
  ) -> EntityIter<'a> {
    match self {
      EntitySelector::Name(name) => {
        for ent in entities.iter() {
          if let Some(p) = ent.as_player() {
            if p.username() == &name {
              return EntityIter::Player(Some(p.clone()));
            }
          }
        }
        EntityIter::Player(None)
      }
      EntitySelector::Entities(props) => EntityIter::Entities { props, iter: entities.iter() },
      EntitySelector::Players(props) => EntityIter::Players { props, iter: entities.iter() },
      EntitySelector::Runner => EntityIter::Player(runner.cloned()),
      EntitySelector::Closest(_props) => {
        // TODO: Filter this by props
        if let Some(runner) = runner {
          let mut min_dist = f64::MAX;
          let mut found_player = None;
          for ent in entities.iter() {
            if let Some(p) = ent.as_player() {
              let dist = p.pos().dist_squared(runner.pos());
              if dist < min_dist {
                min_dist = dist;
                found_player = Some(p.clone());
              }
            }
          }
          EntityIter::Player(found_player)
        } else {
          EntityIter::Player(None)
        }
      }
      EntitySelector::Random(_props) => {
        // TODO: Implement
        EntityIter::Player(None)
      }
    }
  }
}

impl<'a> Iterator for EntityIter<'a> {
  type Item = EntityRef<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      EntityIter::Player(p) => p.take().map(EntityRef::Player),
      EntityIter::Entities { props: _, iter } => {
        // TODO: Read props, and filter this iterator based on this properties.
        iter.next()
      }
      EntityIter::Players { props: _, iter } => {
        // TODO: Read props, and filter this iterator based on this properties.
        loop {
          let ent = iter.next()?;
          if ent.as_player().is_none() {
            continue;
          }
          break Some(ent);
        }
      }
    }
  }
}

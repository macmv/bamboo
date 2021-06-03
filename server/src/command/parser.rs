/// A string parsing type. Used only in [`Parser::String`].
#[derive(Debug, Clone)]
pub enum StringType {
  /// Matches a single word.
  SingleWord,
  /// Matches either a single word, or a phrase in double quotes. Quotes can be
  /// inserted in the string with `\"`.
  QuotablePhrase,
  /// Matches all remaining text in the command. Quotes are not interpreted.
  GreedyPhrase,
}

/// This is a command argument parser. All of the information for this comes
/// from [wiki.vg](https://wiki.vg/Command_Data). They have a great collection
/// of data for all of this stuff, and this entire server wouldn't be possible
/// without them.
#[derive(Debug, Clone)]
pub enum Parser {
  // Simple types:
  /// True or false.
  Bool,
  /// A double, with optional min and max values.
  Double { min: Option<f64>, max: Option<f64> },
  /// A float, with optional min and max values.
  Float { min: Option<f32>, max: Option<f32> },
  /// An int, with optional min and max values.
  Integer { min: Option<i32>, max: Option<i32> },
  /// A string. See [`StringType`] for details on how this is parsed.
  String(StringType),
  /// An entity. If `single` is set, then this can only match one entity (things
  /// like `@e` or `@a` are not allowed). If players is set, then matching
  /// players (with either a username or `@a`) is allowed.
  Entity { single: bool, players: bool },
  /// A user that is on the current scoreboard. With the scoreboard system that
  /// sugarcane has, this doesn't make that much sense.
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

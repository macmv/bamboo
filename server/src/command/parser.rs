#[derive(Debug, Clone)]
pub enum StringType {
  SingleWord,
  QuotablePhrase,
  GreedyPhrase,
}

#[derive(Debug, Clone)]
pub enum Parser {
  Bool,                                           // True or false
  Double { min: Option<f64>, max: Option<f64> },  // A double with optional min and max
  Float { min: Option<f32>, max: Option<f32> },   // A float with optional min and max
  Integer { min: Option<i32>, max: Option<i32> }, // An int with optional min and max
  String(StringType),                             // A string
  Entity { single: bool, players: bool },         // An entity. Can be things like @e, or username
  ScoreHolder { multiple: bool },                 // A user that is on the current scoreboard

  GameProfile,              // Player, online or not. Can also use a selector.
  BlockPos,                 // location, represented as 3 numbers (which must be integers)
  ColumnPos,                // column location, represented as 3 numbers (which must be integers)
  Vec3,                     // A location, represented as 3 numbers
  Vec2,                     // A location, represented as 2 numbers
  BlockState,               // A block state, optionally including NBT and state information.
  BlockPredicate,           // A block, or a block tag.
  ItemStack,                // An item, optionally including NBT.
  ItemPredicate,            // An item, or an item tag.
  Color,                    // Chat color. One of the names from Chat#Colors, or reset.
  Component,                // A JSON Chat component.
  Message,                  // A regular message, potentially including selectors.
  Nbt,                      // An NBT value, parsed using JSON-NBT rules.
  NbtPath,                  // A path within an NBT value, allowing for array and member accesses.
  Objective,                // A scoreboard objective.
  ObjectiveCriteria,        // A single score criterion.
  Operation,                // A scoreboard operator.
  Particle,                 // A particle effect
  Rotation,                 // angle, represented as 2 floats
  Angle,                    // A single float
  ScoreboardSlot,           // Scoreboard display position slot. list, sidebar, belowName, etc
  Swizzle,                  // A collection of up to 3 axes.
  Team,                     // The name of a team. Parsed as an unquoted string.
  ItemSlot,                 // A name for an inventory slot.
  ResourceLocation,         // An Identifier.
  MobEffect,                // A potion effect.
  Function,                 // A function.
  EntityAnchor,             // entity anchor related to the facing argument
  Range { decimals: bool }, // A range of values with a min and a max.
  IntRange,                 // An integer range of values with a min and a max.
  FloatRange,               // A floating-point range of values with a min and a max.
  ItemEnchantment,          // Represents a item enchantment.
  EntitySummon,             // Represents an entity summon.
  Dimension,                // Represents a dimension.
  Uuid,                     // Represents a UUID value.
  NbtTag,                   // Represents a partial nbt tag, usable in data modify command.
  NbtCompoundTag,           // Represents a full nbt tag.
  Time,                     // Represents a time duration.
  Modid,                    // A forge mod id
  Enum,                     // A enum class to use for suggestion. Added by Minecraft Forge.
}

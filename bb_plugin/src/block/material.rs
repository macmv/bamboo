#[rustfmt::skip]
/// A block material. In vanilla, there are a bunch of these. However, the
/// prismarine data limits us to just a few options. This will be updated in the
/// future, to match a simpler version of the 1.8 materials.
///
/// Here are the 1.8 materials:
///
/// Key:
/// - Mat: The named material, according to MCP 1.8
/// - Color: The color name, according to MCP 1.8. This is what will show up on
///   maps
/// - Burn: If there is an `x` in this column, then this type of block can be
///   lit on fire.
/// - Requires Tool: If there is an `x` in this column, then the block will only
///   drop items if broken by the correct tool.
/// - Piston Behavior: This is what happens when the block is pushed by a piston.
///   The block will either behave like normal (get pushed to the side), or it
///   will be immovable (the piston will not extend), or it will be destroyed (the
///   piston will extend, and the block will drop as an item).
/// - Translucent: If there is an `x` in this column, then this block is considered
///   transparent by the game. This means the clients can see through it, mobs
///   cannot spawn ontop of it, etc.
/// - Replaceable: If there is an `x` in this column, then the block will be
///   replaced when right-clicked on. The normal operation is to have the new block
///   be placed next to the block you click on. If the block is replaceable (for
///   example, tall grass) then right clicking on it will just place a new block
///   in the place of the block you clicked on.
/// - Adventure Mode Exempt: This will allow you to modify these blocks in adventure
///   mode.
///
/// | Material     | Color   | Burn | Requires Tool | Piston Behavior | Translucent | Replaceable | Adventure Mode Exempt |
/// |--------------|---------|------|---------------|-----------------|-------------|-------------|-----------------------|
/// | air          | air     |      |               |                 |             |             |                       |
/// | grass        | grass   |      |               |                 |             |             |                       |
/// | ground       | dirt    |      |               |                 |             |             |                       |
/// | wood         | wood    | x    |               |                 |             |             |                       |
/// | rock         | stone   |      | x             |                 |             |             |                       |
/// | iron         | iron    |      | x             |                 |             |             |                       |
/// | anvil        | iron    |      | x             | immovable       |             |             |                       |
/// | water        | water   |      |               | destroy         |             |             |                       |
/// | lava         | tnt     |      |               | destroy         |             |             |                       |
/// | leaves       | foliage | x    |               | destroy         | x           |             |                       |
/// | plants       | foliage |      |               | destroy         |             |             |                       |
/// | vine         | foliage | x    |               | destroy         |             | x           |                       |
/// | sponge       | yellow  |      |               |                 |             |             |                       |
/// | cloth        | cloth   | x    |               |                 |             |             |                       |
/// | fire         | air     |      |               | destroy         |             |             |                       |
/// | sand         | sand    |      |               |                 |             |             |                       |
/// | circuits     | air     |      |               | destroy         |             |             |                       |
/// | carpet       | cloth   | x    |               |                 |             |             |                       |
/// | glass        | air     |      |               |                 | x           |             | x                     |
/// | redstone     | air     |      |               |                 |             |             | x                     |
/// | tnt          | tnt     | x    |               |                 | x           |             |                       |
/// | coral        | foliage |      |               | destroy         |             |             |                       |
/// | ice          | ice     |      |               |                 | x           |             | x                     |
/// | packedIce    | ice     |      |               |                 |             |             | x                     |
/// | snow         | snow    |      |               |                 | x           | x           |                       |
/// | crafted snow | snow    |      | x             |                 |             |             |                       |
/// | cactus       | foliage |      |               | destroy         | x           |             |                       |
/// | clay         | clay    |      |               |                 |             |             |                       |
/// | gourd        | foliage |      |               | destroy         |             |             |                       |
/// | dragon egg   | foliage |      |               | destroy         |             |             |                       |
/// | portal       | air     |      |               | immovable       |             |             |                       |
/// | cake         | air     |      |               | destroy         |             |             |                       |
/// | web          | cloth   |      | x             | destroy         |             |             |                       |
/// | piston       | stone   |      |               | immovable       |             |             |                       |
/// | barrier      | air     |      | x             | destroy         |             |             |                       |
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Material {
  Air,
  // Required for server to not desync things
  ReplaceablePlant,
  Ice,
  DenseIce,

  // Nice to have, not needed
  Soil,
  Stone,
  Wood,
  NetherWood,
  Organic,
  SolidOrganic,
  NetherShoots,
  Earth,
  Plant,
  Water,
  Lava,
  Sand,
  Leaves,
  Sponge,
  Glass,
  Metal,
  Wool,
  Part,
  Piston,
  Cobweb,
  Seagrass,
  UnderwaterPlant,
  Egg,
  Snow,
  SnowBlock,
  SnowPowder,
  Decoration,

  // Makes everything simpler (especially for weird materials that I don't care about).
  Unknown,
}

impl Material {
  pub fn is_replaceable(&self) -> bool { matches!(self, Material::ReplaceablePlant) }
  pub fn slipperiness(&self) -> f32 {
    match self {
      Material::Ice | Material::DenseIce => 0.98,
      _ => 0.6,
    }
  }
  pub fn requires_tool(&self) -> bool {
    matches!(self, Material::Stone | Material::Snow | Material::SnowBlock)
  }
}

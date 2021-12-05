use crate::dl;
use serde::{de, de::Visitor, Deserialize, Deserializer};
use std::{fmt, fs, io, path::Path};

mod gen;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("block"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: BlockDef = dl::get("blocks", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &out_dir.join("block"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockDef {
  blocks: Vec<Block>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Block {
  /// The id of the block.
  id:               u32,
  /// The name id, used everywhere imporant.
  name:             String,
  /// The name used in lang files.
  unlocalized_name: String,
  /// The full class of the block.
  class:            String,

  /// The enum name of the material.
  material:  Material,
  /// The enum name of the map color. Defaults to the map color of the material.
  map_color: String,

  /// The explosion resistance of the block.
  resistance: f32,
  /// The time it takes to mine this block.
  hardness:   f32,

  /// The amount of light this block emits. Will be a number from 0 to 15. This
  /// is zero for most blocks, but will be set for things like torches.
  luminance:    u8,
  /// The slipperiness factor. If set to 0, then this is a normal block.
  /// Otherwise, this is some factor used for ice. Currently, it is always 0.98
  /// for ice, and 0 for everything else.
  slipperiness: f32,

  /// Set when this block doesn't have a hitbox.
  no_collision: bool,
  /// Enum variant of the sound this block makes when walked on.
  sound_type:   String,

  /// A list of items this block drops.
  drops: Vec<String>,

  /// A list of all the properties on this block. If the states are empty, there
  /// is a single valid state for this block, which has no properties. See the
  /// [`State`] docs for more.
  properties: Vec<Prop>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Prop {
  /// The name of this property. This will be something like `rotation` or
  /// `waterlogged`, for example.
  name: String,

  /// The possible values of this state.
  kind: PropKind,
}

#[derive(Debug, Clone, Deserialize)]
pub enum PropKind {
  /// A boolean property. This can either be `true` or `false`.
  Bool,
  /// An enum property. This can be any of the given values.
  Enum(Vec<String>),
  /// A number property. This can be anything from `min..=max`, where `max` is
  /// the inclusive end of the range. The start is normally zero, but can
  /// sometimes be one.
  Int { min: u32, max: u32 },
}

impl<'de> Deserialize<'de> for Material {
  fn deserialize<D>(deserializer: D) -> Result<Material, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct MatVisitor;
    impl<'de> Visitor<'de> for MatVisitor {
      type Value = Material;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a material string")
      }

      fn visit_str<E>(self, name: &str) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        Ok(Material::from_kind(name))
      }
    }
    deserializer.deserialize_str(MatVisitor)
  }
}

macro_rules! mat {
  ($($name:expr => $kind:ident,)*) => {
    #[derive(Debug, Clone)]
    pub enum Material {
      Unknown,
      $($kind,)*
    }

    impl Material {
      pub fn from_kind(kind: &str) -> Material {
        match kind {
          $($name => Self::$kind,)*
          _ => Self::Unknown,
        }
      }
    }
  }
}

mat! {
  "AIR" => Air,
  "STONE" => Stone,
}

impl Default for Material {
  fn default() -> Self {
    Material::Air
  }
}

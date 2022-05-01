use crate::dl;
use serde::{de, de::Visitor, Deserialize, Deserializer};
use std::{fmt, fs, io, path::Path};

mod cross;
mod gen;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("block"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let mut def: BlockDef = dl::get("blocks", ver);
      for block in &mut def.blocks {
        block.properties.sort_unstable_by(|a, b| a.name.cmp(&b.name));
      }
      (ver, def)
    })
    .collect();
  gen::generate(versions, &out_dir.join("block"))?;
  Ok(())
}

#[cfg(test)]
#[test]
fn test_all() {
  println!("generating versions...");
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: BlockDef = dl::get("blocks", ver);
      (ver, def)
    })
    .collect();
  println!("testing...");
  gen::test(versions);
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockDef {
  blocks: Vec<Block>,
}

/// A block in any version. This has `#[allow(dead_code)]` because this is a
/// 1-to-1 with the json data, and I don't want to forget about information
/// included in the json.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
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
  drops: Vec<ItemDrop>,

  /// All of the tags for this block. Each tag starts with `#`, and does not
  /// include `minecraft:`.
  tags: Vec<String>,

  /// A list of all the properties on this block. If the states are empty, there
  /// is a single valid state for this block, which has no properties. See the
  /// [`State`] docs for more.
  properties: Vec<Prop>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDrop {
  /// The item name of this drop.
  item: String,
  /// The minimum amount that can be dropped (inclusive).
  min:  i32,
  /// The maximum amount that can be dropped (inclusive).
  max:  i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Prop {
  /// The name of this property. This will be something like `rotation` or
  /// `waterlogged`, for example.
  name: String,

  /// The possible values of this state.
  kind: PropKind,

  /// The default index of this property.
  default: PropValue,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PropValue {
  /// A boolean property. This can either be `true` or `false`.
  Bool(bool),
  /// An enum property. This can be any of the given values.
  Enum(String),
  /// A number property. This can be anything from `min..=max`, where `max` is
  /// the inclusive end of the range. The start is normally zero, but can
  /// sometimes be one.
  Int(u32),
}

impl PropValue {
  fn to_src(&self) -> String {
    match self {
      Self::Bool(v) => format!("PropValue::Bool({})", v),
      Self::Enum(v) => format!("PropValue::Enum(\"{}\")", v),
      Self::Int(v) => format!("PropValue::Int({})", v),
    }
  }
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
  // Required for server to not desync things
  "REPLACEABLE_PLANT" => ReplaceablePlant,
  "ICE" => Ice,
  "DENSE_ICE" => DenseIce,

  // Nice to have, not needed
  "SOIL" => Soil,
  "STONE" => Stone,
  "WOOD" => Wood,
  "NETHER_WOOD" => NetherWood,
  "ORGANIC" => Organic,
  "SOLID_ORGANIC" => SolidOrganic,
  "NETHER_SHOOTS" => NetherShoots,
  "EARTH" => Earth,
  "PLANT" => Plant,
  "WATER" => Water,
  "LAVA" => Lava,
  "SAND" => Sand,
  "LEAVES" => Leaves,
  "SPONGE" => Sponge,
  "GLASS" => Glass,
  "METAL" => Metal,
  "WOOL" => Wool,
  "PART" => Part,
  "PISTON" => Piston,
  "COBWEB" => Cobweb,
  "SEAGRASS" => Seagrass,
  "UNDERWATER_PLANT" => UnderwaterPlant,
  "EGG" => Egg,
  "SNOW" => Snow,
  "SNOW_BLOCK" => SnowBlock,
  "SNOW_POWDER" => SnowPowder,
  "DECORATION" => Decoration,
}

impl Default for Material {
  fn default() -> Self { Material::Air }
}

#[derive(Debug, Clone)]
pub struct State {
  props: Vec<StateProp>,
}

#[derive(Debug, Clone)]
pub struct StateProp {
  name: String,
  kind: StatePropKind,
}

#[derive(Debug, Clone)]
pub enum StatePropKind {
  Bool(bool),
  Enum(usize, String),
  Int(i32),
}

impl Prop {
  /// Number of states of this property. The state of this should never reach
  /// the value (it works the same way as an array length).
  pub fn len(&self) -> u32 {
    match &self.kind {
      PropKind::Bool => 2,
      PropKind::Enum(v) => v.len() as u32,
      PropKind::Int { min, max } => (max - min) + 1,
    }
  }

  pub fn state(&self, id: u32) -> StateProp {
    StateProp {
      name: self.name.clone(),
      kind: match &self.kind {
        PropKind::Bool => StatePropKind::Bool(id != 0),
        PropKind::Enum(v) => StatePropKind::Enum(id as usize, v[id as usize].clone()),
        PropKind::Int { .. } => StatePropKind::Int(id as i32),
      },
    }
  }
}

impl Block {
  pub fn all_states(&self) -> Vec<State> {
    if self.properties.is_empty() {
      return vec![State { props: vec![] }];
    }
    let mut states = vec![];
    let mut prop_ids = vec![0; self.properties.len()];
    'all: loop {
      states.push(State {
        props: prop_ids.iter().enumerate().map(|(i, id)| self.properties[i].state(*id)).collect(),
      });
      prop_ids[0] += 1;
      for i in 0..prop_ids.len() {
        if prop_ids[i] >= self.properties[i].len() {
          if i >= prop_ids.len() - 1 {
            break 'all;
          }
          prop_ids[i] = 0;
          prop_ids[i + 1] += 1;
        } else {
          break;
        }
      }
    }
    states
  }
}

impl State {
  pub fn prop(&self, name: &str) -> &StateProp {
    self
      .props
      .iter()
      .find(|p| p.name == name)
      .unwrap_or_else(|| panic!("could not find property {}. valid properties: {:?}", name, self))
  }
  pub fn enum_prop(&self, name: &str) -> &str {
    let p = self.prop(name);
    match &p.kind {
      StatePropKind::Enum(_, name) => name,
      _ => panic!("not an enum: {:?}", p),
    }
  }
  pub fn bool_prop(&self, name: &str) -> bool {
    let p = self.prop(name);
    match &p.kind {
      StatePropKind::Bool(v) => *v,
      _ => panic!("not a bool: {:?}", p),
    }
  }
  pub fn int_prop(&self, name: &str) -> i32 {
    let p = self.prop(name);
    match &p.kind {
      StatePropKind::Int(v) => *v,
      _ => panic!("not an int: {:?}", p),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_all_states() {
    let b = Block { properties: vec![], ..Default::default() };
    assert_eq!(b.all_states().len(), 1);

    let b = Block {
      properties: vec![Prop {
        name:    "".into(),
        kind:    PropKind::Enum(vec!["a".into(), "b".into(), "c".into()]),
        default: PropValue::Int(0),
      }],
      ..Default::default()
    };
    assert_eq!(b.all_states().len(), 3);

    let b = Block {
      properties: vec![Prop {
        name:    "".into(),
        kind:    PropKind::Bool,
        default: PropValue::Int(0),
      }],
      ..Default::default()
    };
    assert_eq!(b.all_states().len(), 2);

    let b = Block {
      properties: vec![
        Prop { name: "".into(), kind: PropKind::Bool, default: PropValue::Int(0) },
        Prop { name: "".into(), kind: PropKind::Bool, default: PropValue::Int(0) },
      ],
      ..Default::default()
    };
    assert_eq!(b.all_states().len(), 4);

    let b = Block {
      properties: vec![
        Prop {
          name:    "".into(),
          kind:    PropKind::Enum(vec!["a".into(), "b".into(), "c".into()]),
          default: PropValue::Int(0),
        },
        Prop { name: "".into(), kind: PropKind::Bool, default: PropValue::Int(0) },
      ],
      ..Default::default()
    };
    assert_eq!(b.all_states().len(), 6);

    let b = Block {
      properties: vec![Prop {
        name:    "".into(),
        kind:    PropKind::Int { min: 0, max: 1 },
        default: PropValue::Int(0),
      }],
      ..Default::default()
    };
    assert_eq!(b.all_states().len(), 2);
  }
}

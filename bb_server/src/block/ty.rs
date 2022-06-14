use super::{CustomKind, Material, TypeStore};
use std::{collections::HashMap, error::Error, fmt, str::FromStr};

pub(super) const STATE_PROPS_LEN: usize = 8;

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type<'a> {
  pub(super) kind: Kind,
  state:           u32,
  props:           &'a [Prop],
  // TODO: Make sure there aren't more than 8 properties for any block.
  state_props:     [u32; STATE_PROPS_LEN],
}

#[derive(Debug, Clone, Error, PartialEq)]
#[error("no such property {name}, valid properties are: {props:?}")]
pub struct PropError {
  name:  String,
  props: Vec<Prop>,
}

impl TypeStore {
  pub fn ty(&self) -> Type {
    Type {
      kind:        self.kind,
      state:       self.state,
      props:       &self.props,
      state_props: self.state_props,
    }
  }
}

impl Type<'_> {
  /// Returns the type for air.
  pub const fn air() -> Type<'static> {
    Type {
      kind:        Kind::Air,
      state:       0,
      props:       &[],
      state_props: [0; STATE_PROPS_LEN],
    }
  }
  /// Converts this type into a type that doesn't have a lifetime. Used when
  /// sending a type to another thread, or somewhere that needs a longer
  /// lifetime.
  pub fn to_store(&self) -> TypeStore {
    TypeStore {
      kind:        self.kind,
      state:       self.state,
      props:       self.props.to_vec(),
      state_props: self.state_props,
    }
  }
  /// Returns the block kind that this state comes from.
  pub const fn kind(&self) -> Kind { self.kind }
  /// Gets the block id of this type. This id is for the latest version of the
  /// game.
  pub fn id(&self) -> u32 {
    let mut id = 0;
    for (p, sid) in self.props.iter().zip(self.state_props) {
      id *= p.len() as u32;
      id += sid;
    }
    self.state + id
  }
  pub fn prop(&self, name: &str) -> PropValue<'_> {
    self.try_prop(name).unwrap_or_else(|e| panic!("{e}"))
  }
  pub fn try_prop(&self, name: &str) -> Result<PropValue<'_>, PropError> {
    let mut idx = None;
    for (i, p) in self.props.iter().enumerate() {
      if p.name == name {
        idx = Some(i);
        break;
      }
    }
    if let Some(idx) = idx {
      let state = self.state_props[idx];
      Ok(match self.props[idx].kind {
        PropKind::Bool => match state {
          0 => PropValue::Bool(true),
          _ => PropValue::Bool(false),
        },
        PropKind::Enum(values) => PropValue::Enum(values[state as usize]),
        PropKind::Int { min, max } => PropValue::Int((state + min).min(max)),
      })
    } else {
      Err(PropError { name: name.into(), props: self.props.to_vec() })
    }
  }
  pub fn set_prop<'a>(&mut self, name: &str, val: impl Into<PropValue<'a>>) {
    let mut idx = None;
    for (i, p) in self.props.iter().enumerate() {
      if p.name == name {
        idx = Some(i);
        break;
      }
    }
    if let Some(idx) = idx {
      let val = val.into();
      if val.is(&self.props[idx].kind) {
        self.state_props[idx] = val.id(&self.props[idx].kind);
      } else {
        panic!(
          "the given property {:?} is not compatible with property {:?}",
          val, self.props[idx]
        );
      }
    } else {
      panic!("no such property {}, valid properties are {:?}", name, self.props);
    }
  }
  pub fn with<'a>(mut self, name: &str, val: impl Into<PropValue<'a>>) -> Self {
    self.set_prop(name, val);
    self
  }

  pub fn prop_at(&self, name: &str) -> Option<&Prop> {
    self.props.iter().find(|prop| prop.name == name)
  }
  pub fn props(&self) -> HashMap<String, String> {
    self
      .props
      .iter()
      .enumerate()
      .map(|(i, prop)| (prop.name.into(), prop.kind.name_at(self.state_props[i])))
      .collect()
  }
}
impl fmt::Display for Type<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.kind().to_str())?;
    let mut all_props: Vec<_> = self.props().into_iter().collect();
    all_props.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    if !all_props.is_empty() {
      write!(f, "[")?;
      for (i, (key, val)) in all_props.iter().enumerate() {
        write!(f, "{key}={val}")?;
        if i != all_props.len() - 1 {
          write!(f, ",")?;
        }
      }
      write!(f, "]")?;
    }
    Ok(())
  }
}

#[derive(Debug)]
pub struct InvalidBlock(String);

impl fmt::Display for InvalidBlock {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid block name: {}", self.0)
  }
}

impl Error for InvalidBlock {}

include!(concat!(env!("OUT_DIR"), "/block/ty.rs"));

/// A kind of bounding box. This is from prismarine data. It is not very
/// helpful, and will be replaced when I have a better data source.
#[derive(Debug)]
#[non_exhaustive]
pub enum BoundingBoxKind {
  Empty,
  Block,
}

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  /// The kind for this data.
  pub kind:         Kind,
  /// The name of this block. This is something like `grass_block`.
  pub name:         &'static str,
  /// The material used to make this block. This controls things like map color,
  /// sound, what tool breaks the block, etc. Prismarine doesn't have a very
  /// good material value, so this needs to be updated to more complete data.
  pub material:     Material,
  /// Amount of time it takes to break this block.
  pub hardness:     f32,
  /// How difficult this is to break with an explosion.
  pub resistance:   f32,
  /// A list of item ids this block can drop.
  pub drops:        &'static [ItemDrop],
  /// If this is true, then clients can (at least partially) see through this
  /// block.
  pub transparent:  bool,
  /// This is how much light this block removes. A value of 15 means it blocks
  /// all light, and a value of 0 means it blocks no light.
  pub filter_light: u8,
  /// The amount of light this block emits (0-15).
  pub emit_light:   u8,
  /// The kind of bounding box this block has.
  pub bounding_box: BoundingBoxKind,

  /// The latest version state id. This is the lowest possible state for this
  /// block. It is used to offset the state calculation for properties.
  pub state:     u32,
  /// A list of vanilla tags for this block. Plugins should be able to add tags
  /// in the future. These tags don't include `minecraft:` at the start.
  pub tags:      &'static [&'static str],
  /// All the properties on this block. These are stored so that it is easy to
  /// convert a single property on a block.
  props:         &'static [Prop],
  /// The default type. Each value is an index into that property.
  default_props: &'static [PropValue<'static>],
}

/// A possible item drop for a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDrop {
  pub item: &'static str,
  pub min:  i32,
  pub max:  i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prop {
  pub(super) name: &'static str,
  pub(super) kind: PropKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropKind {
  Bool,
  Enum(&'static [&'static str]),
  Int { min: u32, max: u32 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum PropValue<'a> {
  Bool(bool),
  Enum(&'a str),
  Int(u32),
}

impl PropKind {
  pub fn name_at(&self, id: u32) -> String {
    match self {
      Self::Bool => {
        if id == 0 {
          "true".into()
        } else {
          "false".into()
        }
      }
      Self::Enum(names) => names.get(id as usize).copied().unwrap_or("").into(),
      Self::Int { min, .. } => (min + id).to_string(),
    }
  }
}

impl PropValue<'_> {
  pub fn as_enum(&self) -> &str {
    match self {
      Self::Enum(v) => v,
      _ => "",
    }
  }
  pub(super) fn id(&self, kind: &PropKind) -> u32 {
    match self {
      Self::Bool(v) => {
        if *v {
          0
        } else {
          1
        }
      }
      Self::Enum(v) => match kind {
        PropKind::Enum(variants) => {
          for (i, val) in variants.iter().enumerate() {
            if v == val {
              return i as u32;
            }
          }
          unreachable!();
        }
        _ => unreachable!(),
      },
      Self::Int(v) => match kind {
        PropKind::Int { min, .. } => v - min,
        _ => unreachable!(),
      },
    }
  }
  pub(super) fn is(&self, kind: &PropKind) -> bool {
    match self {
      Self::Bool(_) => matches!(kind, PropKind::Bool),
      Self::Enum(val) => matches!(kind, PropKind::Enum(variants) if variants.contains(val)),
      Self::Int(val) => matches!(kind, PropKind::Int { min, max } if val >= min && val <= max),
    }
  }
}

impl From<bool> for PropValue<'_> {
  fn from(v: bool) -> Self { PropValue::Bool(v) }
}
impl From<u32> for PropValue<'_> {
  fn from(v: u32) -> Self { PropValue::Int(v) }
}
impl<'a> From<&'a str> for PropValue<'a> {
  fn from(v: &'a str) -> Self { PropValue::Enum(v) }
}
impl PartialEq<bool> for PropValue<'_> {
  fn eq(&self, other: &bool) -> bool { matches!(self, PropValue::Bool(v) if v == other) }
}
impl PartialEq<u32> for PropValue<'_> {
  fn eq(&self, other: &u32) -> bool { matches!(self, PropValue::Int(v) if v == other) }
}
impl PartialEq<&str> for PropValue<'_> {
  fn eq(&self, other: &&str) -> bool { matches!(self, PropValue::Enum(v) if v == other) }
}
impl PartialEq<str> for PropValue<'_> {
  fn eq(&self, other: &str) -> bool { matches!(self, PropValue::Enum(v) if *v == other) }
}

impl Data {
  /// Returns the default type for this kind. This is usually what should be
  /// placed down when the user right clicks on a block. Sometimes, for blocks
  /// like stairs or doors, the type that should be placed must be computed when
  /// they place the block, as things like their position/rotation affect which
  /// block gets placed.
  pub fn default_type(&self) -> Type {
    if self.default_props.len() > STATE_PROPS_LEN {
      panic!("Type has too many properties: {:?}", self.props);
    }
    let mut state_props = [0; STATE_PROPS_LEN];
    for (i, p) in self.default_props.iter().enumerate() {
      state_props[i] = p.id(&self.props[i].kind);
    }
    Type { kind: self.kind, state: self.state, props: self.props, state_props }
  }

  /// Returns the type
  pub fn type_from_id(&self, mut id: u32) -> Type {
    let mut state_props = [0; STATE_PROPS_LEN];
    for (i, p) in self.props.iter().enumerate().rev() {
      let len = p.len();
      state_props[i] = id % len;
      id /= len;
    }
    Type { kind: self.kind, state: self.state, props: self.props, state_props }
  }
}

impl Prop {
  #[allow(clippy::len_without_is_empty)]
  pub fn len(&self) -> u32 {
    match self.kind {
      PropKind::Bool => 2,
      PropKind::Enum(v) => v.len() as u32,
      PropKind::Int { min, max } => max - min + 1,
    }
  }

  pub fn from_id(&self, id: u32) -> PropValue<'static> {
    if id >= self.len() {
      panic!("id is {}, but len is {}", id, self.len());
    }
    match self.kind {
      PropKind::Bool => match id {
        0 => PropValue::Bool(true),
        1 => PropValue::Bool(false),
        _ => unreachable!(),
      },
      PropKind::Enum(v) => PropValue::Enum(v[id as usize]),
      PropKind::Int { min, .. } => PropValue::Int(id + min),
    }
  }

  pub fn id_of(&self, val: &PropValue) -> u32 { val.id(&self.kind) }
}

#[cfg(test)]
mod tests {
  use super::{super::TypeConverter, *};

  #[test]
  fn test_generate() {
    let conv = TypeConverter::new();

    const ID: u32 = 148;
    let ty = conv.get(Kind::OakLeaves).default_type();
    assert_eq!(ty.with("distance", 1).with("persistent", true).id(), ID + 0 + 0 * 2);
    assert_eq!(ty.with("distance", 1).with("persistent", false).id(), ID + 1 + 0 * 2);
    assert_eq!(ty.with("distance", 2).with("persistent", true).id(), ID + 0 + 1 * 2);
    assert_eq!(ty.with("distance", 2).with("persistent", false).id(), ID + 1 + 1 * 2);
    assert_eq!(ty.with("distance", 3).with("persistent", true).id(), ID + 0 + 2 * 2);
    assert_eq!(ty.with("distance", 3).with("persistent", false).id(), ID + 1 + 2 * 2);
    assert_eq!(ty.id(), ID + 1 + 6 * 2);
  }
}

use super::{ty::STATE_PROPS_LEN, Type};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CustomKind {
  pub(super) kind: u32,
  pub(super) zero: u32,
}

impl CustomKind {
  pub const fn kind_id(&self) -> u32 { self.kind }
  pub const fn zero_state(&self) -> u32 { self.zero }
}

pub struct CustomBlockBuilder {
  name:  String,
  props: Vec<(String, CustomPropWithDefault)>,
}

impl CustomBlockBuilder {
  pub fn new(name: impl Into<String>) -> Self {
    CustomBlockBuilder { name: name.into(), props: vec![] }
  }

  pub fn prop(mut self, name: impl Into<String>, prop: impl Into<CustomPropWithDefault>) -> Self {
    self.add_prop(name, prop);
    self
  }
  pub fn add_prop(&mut self, name: impl Into<String>, prop: impl Into<CustomPropWithDefault>) {
    self.props.push((name.into(), prop.into()));
  }

  pub fn build<'a>(
    mut self,
    mapper: impl Fn(&HashMap<String, CustomPropValue>) -> Type<'a>,
  ) -> CustomData {
    self.props.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    let mut props = Vec::with_capacity(self.props.len());
    let mut defaults = Vec::with_capacity(self.props.len());
    for (name, prop) in self.props {
      props.push(CustomProp { name, kind: prop.kind });
      defaults.push(prop.default);
    }
    let mut vanilla_states = vec![];
    let mut indices = vec![0; props.len()];
    let mut tmp = HashMap::new();
    // These are all the base properties
    for i in 0..indices.len() {
      tmp.insert(props[i].name.clone(), props[i].from_id(indices[i]));
    }
    loop {
      let mut incremented = false;
      for i in 0..indices.len() {
        // Any time we update `indices`, we update `tmp`, so that the `mapper` call
        // below gets the correct properties.
        if indices[i] >= props[i].len() - 1 {
          indices[i] = 0;
          tmp.insert(props[i].name.clone(), props[i].from_id(indices[i]));
        } else {
          indices[i] += 1;
          tmp.insert(props[i].name.clone(), props[i].from_id(indices[i]));
          incremented = true;
          break;
        }
      }
      if !incremented {
        break;
      }
      vanilla_states.push(mapper(&tmp).id());
    }
    CustomData {
      kind: CustomKind { kind: 0, zero: 0 },
      name: self.name,
      props,
      default_props: defaults,
      state: 0,
      vanilla_states,
    }
  }
}

pub struct CustomPropWithDefault {
  kind:    CustomPropKind,
  default: CustomPropValue,
}

impl<const N: usize> From<([&str; N], &str)> for CustomPropWithDefault {
  fn from(t: ([&str; N], &str)) -> CustomPropWithDefault {
    if !t.0.contains(&t.1) {
      panic!("invalid property, default must be in properties list");
    }
    CustomPropWithDefault {
      kind:    CustomPropKind::Enum(t.0.into_iter().map(Into::into).collect()),
      default: CustomPropValue::Enum(t.1.into()),
    }
  }
}

impl From<bool> for CustomPropWithDefault {
  fn from(v: bool) -> CustomPropWithDefault {
    CustomPropWithDefault { kind: CustomPropKind::Bool, default: CustomPropValue::Bool(v) }
  }
}

use std::ops::RangeInclusive;
impl From<(RangeInclusive<u32>, u32)> for CustomPropWithDefault {
  fn from(t: (RangeInclusive<u32>, u32)) -> CustomPropWithDefault {
    if !t.0.contains(&t.1) {
      panic!("invalid property, default must be within the property range");
    }
    CustomPropWithDefault {
      kind:    CustomPropKind::Int { min: *t.0.start(), max: *t.0.end() },
      default: CustomPropValue::Int(t.1),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::block;

  #[test]
  fn build_block() {
    let vanilla = super::super::TypeConverter::new();
    let block = CustomData::builder("my_block")
      .prop("axis", (["x", "y", "z"], "y"))
      .prop("oak", false)
      .prop("distance", (0..=5, 0))
      .build(|ty| match (ty["axis"].str(), ty["oak"].bool()) {
        ("x", false) => vanilla.ty(block::Kind::SpruceLog).with("axis", "x"),
        ("y", false) => vanilla.ty(block::Kind::SpruceLog).with("axis", "y"),
        ("z", false) => vanilla.ty(block::Kind::SpruceLog).with("axis", "z"),
        ("x", true) => vanilla.ty(block::Kind::OakLog).with("axis", "x"),
        ("y", true) => vanilla.ty(block::Kind::OakLog).with("axis", "y"),
        ("z", true) => vanilla.ty(block::Kind::OakLog).with("axis", "z"),
        _ => unreachable!(),
      });
    dbg!(block);
  }
}

/// A custom block. This can be built using [`CustomBlockBuilder`].
#[derive(Debug)]
pub struct CustomData {
  /// The kind of this data.
  pub kind: CustomKind,
  /// The name of this block.
  pub name: String,

  /// The properties of this block.
  props:         Vec<CustomProp>,
  /// The default properties. This must be the same length as `props`.
  default_props: Vec<CustomPropValue>,

  /// The latest version state id. This is the lowest possible state for this
  /// block. It is used to offset the state calculation for properties.
  state: u32,

  /// Maps every possible property combination to a vanilla block. If props is
  /// empty, this will contain a single element.
  vanilla_states: Vec<u32>,
}

impl CustomData {
  /// Starts creating a new block.
  pub fn builder(name: impl Into<String>) -> CustomBlockBuilder { CustomBlockBuilder::new(name) }
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
    // Type { kind: Kind::Custom(self.kind), state: self.state, props: &self.props,
    // state_props }
    todo!()
  }

  pub(super) fn vanilla_for_state(&self, state: u32) -> u32 { self.vanilla_states[state as usize] }

  /// Returns the minimum state id for this block. This is not the same as the
  /// default state.
  pub(super) fn state_id(&self) -> u32 { self.state }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomProp {
  pub(super) name: String,
  pub(super) kind: CustomPropKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomPropKind {
  Bool,
  Enum(Vec<String>),
  Int { min: u32, max: u32 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum CustomPropValue {
  Bool(bool),
  Enum(String),
  Int(u32),
}

impl CustomPropValue {
  pub fn bool(&self) -> bool {
    match self {
      CustomPropValue::Bool(v) => *v,
      _ => panic!("not a bool: {self:?}"),
    }
  }
  pub fn str(&self) -> &str {
    match self {
      CustomPropValue::Enum(v) => v,
      _ => panic!("not a bool: {self:?}"),
    }
  }
  pub fn int(&self) -> u32 {
    match self {
      CustomPropValue::Int(v) => *v,
      _ => panic!("not a bool: {self:?}"),
    }
  }

  pub fn as_enum(&self) -> &str {
    match self {
      Self::Enum(v) => v,
      _ => "",
    }
  }
  pub(super) fn id(&self, kind: &CustomPropKind) -> u32 {
    match self {
      Self::Bool(v) => {
        if *v {
          0
        } else {
          1
        }
      }
      Self::Enum(v) => match kind {
        CustomPropKind::Enum(variants) => {
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
        CustomPropKind::Int { min, .. } => v - min,
        _ => unreachable!(),
      },
    }
  }
  pub fn is(&self, kind: &CustomPropKind) -> bool {
    match self {
      Self::Bool(_) => matches!(kind, CustomPropKind::Bool),
      Self::Enum(val) => matches!(kind, CustomPropKind::Enum(variants) if variants.contains(val)),
      Self::Int(val) => {
        matches!(kind, CustomPropKind::Int { min, max } if val >= min && val <= max)
      }
    }
  }
}

impl CustomProp {
  #[allow(clippy::len_without_is_empty)]
  pub fn len(&self) -> u32 {
    match &self.kind {
      CustomPropKind::Bool => 2,
      CustomPropKind::Enum(v) => v.len() as u32,
      CustomPropKind::Int { min, max } => max - min + 1,
    }
  }

  pub fn from_id(&self, id: u32) -> CustomPropValue {
    if id >= self.len() {
      panic!("id is {}, but len is {}", id, self.len());
    }
    match &self.kind {
      CustomPropKind::Bool => match id {
        0 => CustomPropValue::Bool(true),
        1 => CustomPropValue::Bool(false),
        _ => unreachable!(),
      },
      CustomPropKind::Enum(v) => CustomPropValue::Enum(v[id as usize].clone()),
      CustomPropKind::Int { min, .. } => CustomPropValue::Int(id + min),
    }
  }
}

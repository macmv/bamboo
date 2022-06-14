use super::{Data, Type};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropValue<'a> {
  Bool(bool),
  Enum(&'a str),
  Int(u32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropValueStore {
  Bool(bool),
  Enum(String),
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
  pub fn id(&self, kind: &PropKind) -> u32 {
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
  pub fn is(&self, kind: &PropKind) -> bool {
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

impl From<String> for PropValueStore {
  fn from(v: String) -> Self { PropValueStore::Enum(v) }
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

impl PartialEq<bool> for PropValueStore {
  fn eq(&self, other: &bool) -> bool { matches!(self, PropValueStore::Bool(v) if v == other) }
}
impl PartialEq<u32> for PropValueStore {
  fn eq(&self, other: &u32) -> bool { matches!(self, PropValueStore::Int(v) if v == other) }
}
impl PartialEq<&str> for PropValueStore {
  fn eq(&self, other: &&str) -> bool { matches!(self, PropValueStore::Enum(v) if v == other) }
}
impl PartialEq<str> for PropValueStore {
  fn eq(&self, other: &str) -> bool { matches!(self, PropValueStore::Enum(v) if *v == other) }
}

impl Data {
  /// Returns the default type for this kind. This is usually what should be
  /// placed down when the user right clicks on a block. Sometimes, for blocks
  /// like stairs or doors, the type that should be placed must be computed when
  /// they place the block, as things like their position/rotation affect which
  /// block gets placed.
  pub fn default_type(&self) -> Type {
    todo!();
    /*
    if self.default_props.len() > STATE_PROPS_LEN {
      panic!("Type has too many properties: {:?}", self.props);
    }
    let mut state_props = [0; STATE_PROPS_LEN];
    for (i, p) in self.default_props.iter().enumerate() {
      state_props[i] = p.id(&self.props[i].kind);
    }
    Type { kind: self.kind, state: self.state, props: &self.props, state_props }
    */
  }

  /// Returns the type
  pub fn type_from_id(&self, _id: u32) -> Type {
    todo!();
    /*
    let mut state_props = [0; STATE_PROPS_LEN];
    for (i, p) in self.props.iter().enumerate().rev() {
      let len = p.len();
      state_props[i] = id % len;
      id /= len;
    }
    Type { kind: self.kind, state: self.state, props: &self.props, state_props }
    */
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

impl PropValueStore {
  pub(super) fn new(prop: bb_ffi::CBlockPropValue) -> Self {
    match prop.into_renum() {
      bb_ffi::CBlockPropValueEnum::Bool(v) => Self::Bool(v.as_bool()),
      bb_ffi::CBlockPropValueEnum::Enum(v) => Self::Enum(v.into_string()),
      bb_ffi::CBlockPropValueEnum::Int(v) => Self::Int(v),
    }
  }
}

impl PropValue<'_> {
  pub fn bool(&self) -> bool {
    match self {
      Self::Bool(v) => *v,
      _ => panic!("not a bool: {self:?}"),
    }
  }
  pub fn str(&self) -> &str {
    match self {
      Self::Enum(v) => v,
      _ => panic!("not an enum: {self:?}"),
    }
  }
  pub fn int(&self) -> u32 {
    match self {
      Self::Int(v) => *v,
      _ => panic!("not an int: {self:?}"),
    }
  }
}

impl PropValueStore {
  pub fn bool(&self) -> bool {
    match self {
      Self::Bool(v) => *v,
      _ => panic!("not a bool: {self:?}"),
    }
  }
  pub fn str(&self) -> &str {
    match self {
      Self::Enum(v) => v,
      _ => panic!("not an enum: {self:?}"),
    }
  }
  pub fn int(&self) -> u32 {
    match self {
      Self::Int(v) => *v,
      _ => panic!("not an int: {self:?}"),
    }
  }
}

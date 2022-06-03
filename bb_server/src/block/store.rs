use super::{ty::STATE_PROPS_LEN, Kind, Prop, PropKind, PropValue, Type};
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeStore {
  pub(super) kind:        Kind,
  pub(super) state:       u32,
  pub(super) props:       Vec<Prop>,
  pub(super) state_props: [u32; STATE_PROPS_LEN],
}

impl fmt::Display for TypeStore {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.ty().fmt(f) }
}
impl TypeStore {
  /// Returns the type for air.
  pub fn air() -> TypeStore {
    TypeStore {
      kind:        Kind::Air,
      state:       0,
      props:       vec![],
      state_props: [0; STATE_PROPS_LEN],
    }
  }
  /// Returns the block kind that this state comes from.
  pub fn kind(&self) -> Kind { self.kind }
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
    let mut idx = None;
    for (i, p) in self.props.iter().enumerate() {
      if p.name == name {
        idx = Some(i);
        break;
      }
    }
    if let Some(idx) = idx {
      let state = self.state_props[idx];
      match self.props[idx].kind {
        PropKind::Bool => match state {
          0 => PropValue::Bool(true),
          _ => PropValue::Bool(false),
        },
        PropKind::Enum(values) => PropValue::Enum(values[state as usize]),
        PropKind::Int { min, max } => PropValue::Int((state + min).min(max)),
      }
    } else {
      panic!("no such property {}, valid properties are {:?}", name, self.props);
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
  pub fn with_prop<'a>(mut self, name: &str, val: impl Into<PropValue<'a>>) -> Self {
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

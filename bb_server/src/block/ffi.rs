use super::{Data, ItemDrop, Prop, PropValue, PropValueStore};
use crate::plugin::wasm::{Env, FromFfi, ToFfi};

use bb_ffi::{CBlockData, CBlockProp, CBlockPropValue, CBlockPropValueEnum, CItemDrop};

impl ToFfi for Data {
  type Ffi = CBlockData;

  fn to_ffi(&self, env: &Env) -> CBlockData {
    CBlockData {
      // TODO: Convert to old id
      kind:         self.kind.id(),
      name:         self.name.to_ffi(env),
      material:     0, // self.material.id(),
      hardness:     self.hardness,
      resistance:   self.resistance,
      drops:        Vec::<ItemDrop>::new().as_slice().to_ffi(env), // self.drops.to_ffi(env),
      transparent:  self.transparent.to_ffi(env),
      filter_light: self.filter_light,
      emit_light:   self.emit_light,

      state:         self.state,
      tags:          Vec::<&str>::new().as_slice().to_ffi(env), // self.tags.to_ffi(),
      props:         Vec::<Prop>::new().as_slice().to_ffi(env), // self.props.to_ffi(),
      default_props: Vec::<u32>::new().as_slice().to_ffi(env),  // self.default_props.to_ffi(),
    }
  }
}

impl ToFfi for ItemDrop {
  type Ffi = CItemDrop;

  fn to_ffi(&self, _: &Env) -> CItemDrop { todo!() }
}

impl ToFfi for Prop {
  type Ffi = CBlockProp;

  fn to_ffi(&self, _: &Env) -> CBlockProp { todo!() }
}

impl ToFfi for PropValue<'_> {
  type Ffi = CBlockPropValue;

  fn to_ffi(&self, env: &Env) -> CBlockPropValue {
    match self {
      Self::Bool(v) => CBlockPropValueEnum::Bool(v.to_ffi(env)),
      Self::Enum(v) => CBlockPropValueEnum::Enum(v.to_ffi(env)),
      Self::Int(v) => CBlockPropValueEnum::Int(v.to_ffi(env)),
    }
    .into_cenum()
  }
}
impl FromFfi for PropValueStore {
  type Ffi = CBlockPropValue;

  fn from_ffi(env: &Env, prop: CBlockPropValue) -> Self {
    match prop.into_renum() {
      CBlockPropValueEnum::Bool(v) => Self::Bool(v.as_bool()),
      CBlockPropValueEnum::Enum(v) => Self::Enum(String::from_ffi(env, v)),
      CBlockPropValueEnum::Int(v) => Self::Int(v),
    }
  }
}

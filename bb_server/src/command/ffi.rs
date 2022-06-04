use super::Arg;
use crate::{
  block,
  plugin::wasm::{Env, ToFfi},
  world::WorldManager,
};
use bb_common::math::FPos;
use bb_ffi::CArg;
use bb_transfer::MessageReader;
use wasmer::Memory;

impl ToFfi for Arg {
  type Ffi = CArg;

  fn to_ffi(&self, env: &Env) -> CArg {
    match self {
      Self::Literal(v) => CArg::new_literal(v.as_str().to_ffi(env)),
      Self::Bool(v) => todo!(),
      Self::Double(v) => todo!(),
      Self::Float(v) => todo!(),
      Self::Int(v) => todo!(),
      Self::String(v) => todo!(),
      Self::BlockPos(v) => todo!(),
      Self::Vec3(x, y, z) => todo!(),
      Self::Vec2(x, y) => todo!(),
      Self::BlockState(v, _, _) => todo!(),
      _ => unimplemented!("command arg to ffi {self:?}"),
    }
  }
}

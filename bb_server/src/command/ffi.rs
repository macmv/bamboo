use super::{Arg, Parser};
use crate::{
  block,
  plugin::wasm::{Env, FromFfi, ToFfi},
  world::WorldManager,
};
use bb_common::math::FPos;
use bb_ffi::{CCommandArg, CCommandParser};
use bb_transfer::MessageReader;
use wasmer::Memory;

impl ToFfi for Arg {
  type Ffi = CCommandArg;

  fn to_ffi(&self, env: &Env) -> CCommandArg {
    use bb_ffi::CCommandArgEnum as A;
    match self {
      Self::Literal(v) => A::Literal(v.as_str().to_ffi(env)),
      Self::Bool(v) => todo!(),
      Self::Double(v) => todo!(),
      Self::Float(v) => A::Float(*v),
      Self::Int(v) => todo!(),
      Self::String(v) => todo!(),
      Self::BlockPos(v) => todo!(),
      Self::Vec3(x, y, z) => todo!(),
      Self::Vec2(x, y) => todo!(),
      Self::BlockState(v, _, _) => todo!(),
      _ => unimplemented!("command arg to ffi {self:?}"),
    }
    .into_cenum()
  }
}

impl FromFfi for Parser {
  type Ffi = CCommandParser;

  fn from_ffi(env: &Env, parser: CCommandParser) -> Self {
    use bb_ffi::CCommandParserEnum as P;
    match parser.into_renum() {
      P::Float { min, max } => {
        Parser::Float { min: Option::from_ffi(env, min), max: Option::from_ffi(env, max) }
      }
      _ => todo!(),
    }
  }
}

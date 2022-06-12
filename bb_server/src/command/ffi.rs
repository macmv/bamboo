use super::{Arg, Parser};
use crate::plugin::wasm::{Env, FromFfi, ToFfi};

use bb_ffi::{CCommandArg, CCommandParser};

impl ToFfi for Arg {
  type Ffi = CCommandArg;

  fn to_ffi(&self, env: &Env) -> CCommandArg {
    use bb_ffi::CCommandArgEnum as A;
    match self {
      Self::Literal(v) => A::Literal(v.as_str().to_ffi(env)),
      Self::Bool(_v) => todo!(),
      Self::Double(_v) => todo!(),
      Self::Float(v) => A::Float(*v),
      Self::Int(_v) => todo!(),
      Self::String(_v) => todo!(),
      Self::BlockPos(_v) => todo!(),
      Self::Vec3(_x, _y, _z) => todo!(),
      Self::Vec2(_x, _y) => todo!(),
      Self::BlockState(_v, _, _) => todo!(),
      _ => unimplemented!("command arg to ffi {self:?}"),
    }
    .into_cenum()
  }
}

impl FromFfi for Parser {
  type Ffi = CCommandParser;

  fn from_ffi(env: &Env, parser: CCommandParser) -> Self {
    use bb_ffi::CCommandParserEnum as P;
    dbg!(parser);
    match parser.into_renum() {
      P::Float { min, max } => {
        Parser::Float { min: Option::from_ffi(env, min), max: Option::from_ffi(env, max) }
      }
      _ => todo!(),
    }
  }
}

use super::Env;
use crate::world::WorldManager;
use bb_common::{
  math::{FPos, Pos},
  version::ProtocolVersion,
};
use bb_ffi::{CBool, CFPos, CList, CPos};
use std::mem;
use wasmer::Memory;

pub trait FromFfi {
  type Ffi;

  fn from_ffi(env: &Env, ffi: Self::Ffi) -> Self;
}

impl FromFfi for Pos {
  type Ffi = CPos;

  fn from_ffi(_: &Env, ffi: CPos) -> Self { Pos { x: ffi.x, y: ffi.y, z: ffi.z } }
}
impl FromFfi for FPos {
  type Ffi = CFPos;

  fn from_ffi(_: &Env, ffi: CFPos) -> Self { FPos { x: ffi.x, y: ffi.y, z: ffi.z } }
}
impl FromFfi for bool {
  type Ffi = CBool;

  fn from_ffi(_: &Env, ffi: CBool) -> Self { ffi.as_bool() }
}
impl<T> FromFfi for Vec<T>
where
  T: wasmer::ValueType,
{
  type Ffi = CList<T>;

  fn from_ffi(env: &Env, ffi: CList<T>) -> Self {
    let view = env.mem().view::<T>();
    let ptr = (ffi.first.offset() as usize) / (mem::size_of::<T>());
    view[ptr..ptr + ffi.len as usize].iter().map(|it| it.get()).collect()
  }
}

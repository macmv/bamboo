use bb_common::math::{FPos, Pos};
use bb_ffi::{CBool, CFPos, CList, CPos};
use std::mem;
use wasmer::Memory;

pub trait FromFfi {
  type Ffi;

  fn from_ffi(memory: &Memory, ffi: Self::Ffi) -> Self;
}

impl FromFfi for Pos {
  type Ffi = CPos;

  fn from_ffi(_: &Memory, ffi: CPos) -> Self { Pos { x: ffi.x, y: ffi.y, z: ffi.z } }
}
impl FromFfi for FPos {
  type Ffi = CFPos;

  fn from_ffi(_: &Memory, ffi: CFPos) -> Self { FPos { x: ffi.x, y: ffi.y, z: ffi.z } }
}
impl FromFfi for bool {
  type Ffi = CBool;

  fn from_ffi(_: &Memory, ffi: CBool) -> Self { ffi.as_bool() }
}
impl<T> FromFfi for Vec<T>
where
  T: wasmer::ValueType,
{
  type Ffi = CList<T>;

  fn from_ffi(mem: &Memory, ffi: CList<T>) -> Self {
    let view = mem.view::<T>();
    let ptr = (ffi.first.offset() as usize) / (mem::size_of::<T>());
    view[ptr..ptr + ffi.len as usize].iter().map(|it| it.get()).collect()
  }
}

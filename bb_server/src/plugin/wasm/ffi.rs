use super::Env;
use crate::world::WorldManager;
use bb_common::{
  math::{FPos, Pos},
  version::ProtocolVersion,
};
use bb_ffi::{CBool, CFPos, CList, CPos, CStr};
use std::mem;
use wasmer::Memory;

pub trait FromFfi {
  type Ffi: wasmer::ValueType;

  fn from_ffi(env: &Env, ffi: Self::Ffi) -> Self;
}
pub trait ToFfi {
  type Ffi: wasmer::ValueType;

  fn to_ffi(&self, env: &Env) -> Self::Ffi;
}

macro_rules! self_to_ffi {
  ( $($ty:ty),* ) => {
    $(
      impl ToFfi for $ty {
        type Ffi = Self;
        fn to_ffi(&self, _: &Env) -> Self::Ffi { *self }
      }
    )*
  }
}

self_to_ffi!(u8, i8, u16, i16, u32, i32);

impl FromFfi for Pos {
  type Ffi = CPos;

  fn from_ffi(_: &Env, ffi: CPos) -> Self { Pos { x: ffi.x, y: ffi.y, z: ffi.z } }
}
impl ToFfi for Pos {
  type Ffi = CPos;

  fn to_ffi(&self, _: &Env) -> CPos { CPos { x: self.x, y: self.y, z: self.z } }
}
impl FromFfi for FPos {
  type Ffi = CFPos;

  fn from_ffi(_: &Env, ffi: CFPos) -> Self { FPos { x: ffi.x, y: ffi.y, z: ffi.z } }
}
impl FromFfi for bool {
  type Ffi = CBool;

  fn from_ffi(_: &Env, ffi: CBool) -> Self { ffi.as_bool() }
}
impl ToFfi for bool {
  type Ffi = CBool;

  fn to_ffi(&self, _: &Env) -> CBool { CBool::new(*self) }
}
impl<T> FromFfi for Vec<T>
where
  T: wasmer::ValueType,
{
  type Ffi = CList<T>;

  fn from_ffi(env: &Env, ffi: CList<T>) -> Self {
    let view = env.mem().view::<T>();
    let ptr = (ffi.first.offset() as usize) / mem::size_of::<T>();
    view[ptr..ptr + ffi.len as usize].iter().map(|it| it.get()).collect()
  }
}
impl<T> ToFfi for &[T]
where
  T: ToFfi,
{
  type Ffi = CList<T::Ffi>;

  fn to_ffi(&self, env: &Env) -> CList<T::Ffi> {
    // Using malloc_store gives us a single copy, but we would need to allocate. So,
    // I just iterate through and write every value after calling to_ffi.
    let ptr = env.malloc_array::<T::Ffi>(self.len() as u32);
    if ptr.offset() == 0 {
      panic!("plugin oom");
    }
    if ptr.offset() as usize + mem::size_of::<T::Ffi>() * self.len()
      > env.mem().data_size() as usize
    {
      panic!("invalid ptr");
    }
    // SAFETY: We just validated all of ptr..ptr + self.len is valid.
    unsafe {
      let data_ptr = env.mem().data_ptr() as *mut T::Ffi;
      for (i, elem) in self.iter().enumerate() {
        let ptr = data_ptr.add(i);
        std::ptr::write(ptr, elem.to_ffi(env));
      }
    }
    CList { first: ptr, len: self.len() as u32 }
  }
}
impl ToFfi for &'_ str {
  type Ffi = CStr;

  fn to_ffi(&self, env: &Env) -> CStr {
    let ptr = env.malloc_array_store(self.as_bytes());
    if ptr.offset() == 0 {
      panic!("plugin oom");
    }
    CStr { ptr, len: self.len() as u32 }
  }
}

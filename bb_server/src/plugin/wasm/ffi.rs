use super::Env;

use bb_common::{
  math::{FPos, Pos},
  util::UUID,
};
use bb_ffi::{CBool, CFPos, CList, COpt, CPos, CStr, CUUID};
use std::mem;

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
      impl FromFfi for $ty {
        type Ffi = Self;
        fn from_ffi(_: &Env, v: Self::Ffi) -> Self { v }
      }
    )*
  }
}

self_to_ffi!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl<T: FromFfi> FromFfi for Option<T> {
  type Ffi = COpt<T::Ffi>;

  fn from_ffi(env: &Env, ffi: COpt<T::Ffi>) -> Self {
    if ffi.present.as_bool() {
      unsafe { Some(T::from_ffi(env, ffi.value.assume_init())) }
    } else {
      None
    }
  }
}
impl<T: ToFfi> ToFfi for Option<T> {
  type Ffi = COpt<T::Ffi>;

  fn to_ffi(&self, env: &Env) -> COpt<T::Ffi> {
    match self {
      Some(v) => COpt { present: CBool::new(true), value: mem::MaybeUninit::new(v.to_ffi(env)) },
      None => COpt { present: CBool::new(false), value: mem::MaybeUninit::uninit() },
    }
  }
}
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
impl ToFfi for FPos {
  type Ffi = CFPos;

  fn to_ffi(&self, _: &Env) -> CFPos { CFPos { x: self.x, y: self.y, z: self.z } }
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
  T::Ffi: std::fmt::Debug,
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
      // Call add on *mut u8, not *mut T::Ffi.
      let data_ptr = env.mem().data_ptr().add(ptr.offset() as usize) as *mut T::Ffi;
      for (i, elem) in self.iter().enumerate() {
        let ptr = data_ptr.add(i);
        std::ptr::write(ptr, elem.to_ffi(env));
      }
    }
    CList { first: ptr, len: self.len() as u32 }
  }
}
impl ToFfi for UUID {
  type Ffi = CUUID;

  fn to_ffi(&self, _env: &Env) -> CUUID {
    CUUID {
      bytes: [
        self.as_u128() as u32,
        (self.as_u128() >> 32) as u32,
        (self.as_u128() >> (2 * 32)) as u32,
        (self.as_u128() >> (3 * 32)) as u32,
      ],
    }
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
impl FromFfi for String {
  type Ffi = CStr;

  fn from_ffi(env: &Env, cstr: CStr) -> String {
    cstr.ptr.get_utf8_string(env.mem(), cstr.len).unwrap()
  }
}

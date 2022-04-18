use std::os::raw::c_char;
#[cfg(feature = "host")]
use wasmer_types::ValueType;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPlayer {
  pub eid: i32,
}

#[cfg(feature = "host")]
unsafe impl ValueType for CPlayer {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CChat {
  pub message: *const c_char,
}

#[cfg(feature = "host")]
unsafe impl ValueType for CChat {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

#[cfg(feature = "host")]
unsafe impl ValueType for CPos {}

extern "C" {
  pub fn broadcast(message: *const CChat);
}

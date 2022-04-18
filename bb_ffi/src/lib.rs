use std::os::raw::c_char;
use wasmer_types::ValueType;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPlayer {
  pub eid: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CChat {
  pub message: *const c_char,
}

unsafe impl ValueType for CChat {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

extern "C" {
  pub fn broadcast(message: *const CChat);
}

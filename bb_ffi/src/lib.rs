use std::{
  ffi::{CStr, CString},
  marker::PhantomData,
  os::raw::c_char,
};

#[repr(C)]
pub struct Player {
  pub eid: i32,
}

#[repr(C)]
pub struct Chat {
  pub message: *const c_char,
}

#[repr(C)]
pub struct Pos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

pub struct Bamboo {
  marker: PhantomData<()>,
}

impl Chat {
  pub fn new(message: String) -> Self {
    Chat { message: CString::new(message).unwrap().into_raw() }
  }
  pub unsafe fn to_str<'a>(&'a self) -> &'a str { CStr::from_ptr(self.message).to_str().unwrap() }
}

pub fn instance() -> Bamboo { Bamboo { marker: PhantomData::default() } }

extern "C" {
  fn broadcast(message: i32);
}

impl Bamboo {
  pub fn broadcast(&self, message: Chat) {
    unsafe {
      broadcast(message.message as i32);
    }
  }
}

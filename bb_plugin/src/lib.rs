use bb_ffi::{CChat, CPlayer, CPos};
use std::{ffi::CString, marker::PhantomData};

pub struct Player {
  ffi: CPlayer,
}
pub struct Chat {
  message: String,
}
pub struct Pos {
  ffi: CPos,
}

pub struct Bamboo {
  marker: PhantomData<()>,
}

impl Chat {
  pub fn new(message: String) -> Self { Chat { message } }
}

pub fn instance() -> Bamboo { Bamboo { marker: PhantomData::default() } }

impl Bamboo {
  pub fn broadcast(&self, message: Chat) {
    unsafe {
      let s = CString::new(message.message.clone()).unwrap();
      let c_chat = CChat { message: s.as_ptr() };
      bb_ffi::broadcast(&c_chat);
    }
  }
}

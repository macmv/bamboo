use bb_ffi::{CChat, CPlayer, CPos};
use std::{ffi::CString, marker::PhantomData};

use bb_common::util::Chat;

pub use bb_common::{math, util};

pub struct Bamboo {
  marker: PhantomData<()>,
}

pub fn instance() -> Bamboo { Bamboo { marker: PhantomData::default() } }

impl Bamboo {
  pub fn broadcast(&self, message: Chat) {
    unsafe {
      let s = CString::new(message.to_codes()).unwrap();
      let c_chat = CChat { message: s.as_ptr() };
      bb_ffi::broadcast(&c_chat);
    }
  }
}

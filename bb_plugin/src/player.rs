use bb_ffi::CUUID;
use std::ffi::CStr;

pub struct Player {
  id: CUUID,
}

impl Player {
  pub fn new(id: CUUID) -> Self { Player { id } }

  pub fn username(&self) -> String {
    unsafe {
      let mut buf = [0; 64];
      // We need null terminator, so we make use this doesn't overwrite the last byte.
      bb_ffi::bb_player_username(&self.id, buf.as_mut_ptr(), buf.len() as u32 - 1);
      CStr::from_ptr(buf.as_ptr() as *const _).to_str().unwrap().into()
    }
  }
}

#![deny(improper_ctypes)]

use bb_ffi_macros::ctype;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CStr {
  #[cfg(feature = "host")]
  pub ptr: wasmer::WasmPtr<u8, wasmer::Array>,
  #[cfg(not(feature = "host"))]
  pub ptr: *const u8,
  pub len: u32,
}

#[cfg(feature = "host")]
impl Copy for CStr {}
#[cfg(feature = "host")]
unsafe impl wasmer::ValueType for CStr {}

#[ctype]
#[derive(Debug)]
pub struct CPlayer {
  pub eid: i32,
}

#[ctype]
#[derive(Debug)]
pub struct CUUID {
  pub bytes: [u32; 4],
}

#[ctype]
#[derive(Debug)]
pub struct CChat {
  pub message: CStr,
}

#[ctype]
#[derive(Debug)]
pub struct CPos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

#[ctype]
#[derive(Debug)]
pub struct CCommand {
  /// The name of this command segment. For literals this is their value.
  /// Doesn't contain a NUL at the end.
  pub name:      CStr,
  /// An enum.
  ///
  /// ```text
  /// 0 -> literal
  /// 1 -> argument
  /// _ -> invalid command
  /// ```
  pub node_type: u8,
  /// This is null for `node_type` of root or literal. Doesn't contain a NUL at
  /// the end.
  pub parser:    CStr,
  /// This is a boolean, but `bool` isn't `ValueType` safe.
  pub optional:  u8,
  /// The children of this command.
  pub children:  CList<CCommand>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct CList<T> {
  /// The pointer to the first element in this list.
  pub first: *const T,
  /// The length of this list.
  pub len:   u32,
}

#[cfg(feature = "host")]
impl<T: Clone> Copy for CList<T> {}
#[cfg(feature = "host")]
unsafe impl<T: Clone> wasmer::ValueType for CList<T> {}

extern "C" {
  /// Logs the given message.
  pub fn bb_log(
    level: u32,
    message_ptr: *const u8,
    message_len: u32,
    target_ptr: *const u8,
    target_len: u32,
    module_path_ptr: *const u8,
    module_path_len: u32,
    file_ptr: *const u8,
    file_len: u32,
    line: u32,
  );

  /// Adds the command to the server.
  pub fn bb_add_command(command: *const CCommand);

  /// Broadcasts the given chat message to all players.
  pub fn bb_broadcast(message: *const CChat);
  /// Writes the player's username into the given buffer. Returns 1
  /// if the username won't fit in the buffer, or if the pointer is
  /// invalid.
  pub fn bb_player_username(player: *const CUUID, buf: *mut u8, len: u32) -> i32;
  /// Returns the current world for this player.
  pub fn bb_player_world(player: *const CUUID) -> i32;
  /// Sends the given chat message to the player.
  pub fn bb_player_send_message(player: *const CUUID, message: *const CChat);

  /// Sets a block in the world. Returns 1 if the block position is invalid.
  pub fn bb_world_set_block(wid: u32, pos: *const CPos, id: u32, version: u32) -> i32;

  /// Returns the number of nanoseconds since this function was called first.
  /// This is used to find the duration of a function.
  pub fn bb_time_since_start() -> u64;
}

/*
use std::fmt;
impl<T> fmt::Pointer for CPtr<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "0x{:08x}", self.as_ptr() as u32)
  }
}
impl<T> fmt::Debug for CPtr<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { fmt::Pointer::fmt(self, f) }
}
*/

impl CStr {
  #[cfg(not(feature = "host"))]
  pub fn new(s: String) -> Self {
    let boxed_str = s.into_boxed_str();
    let s = Box::leak(boxed_str);
    CStr { ptr: s.as_ptr() as _, len: s.len() as u32 }
  }
}

// On the host, this refers to data in wasm, so we don't want to free it.
#[cfg(not(feature = "host"))]
impl Drop for CStr {
  fn drop(&mut self) {
    unsafe {
      String::from_raw_parts(self.ptr as *mut u8, self.len as usize, self.len as usize);
    }
  }
}

impl<T> CList<T> {
  #[cfg(not(feature = "host"))]
  pub fn new(list: Vec<T>) -> Self {
    let boxed_slice = list.into_boxed_slice();
    let slice = Box::leak(boxed_slice);
    Self { first: slice.as_ptr() as _, len: slice.len() as u32 }
  }
  #[cfg(feature = "host")]
  pub fn get_ptr(&self, index: u32) -> Option<*const T> {
    if index < self.len {
      Some(unsafe { self.first.add(index as usize) })
    } else {
      None
    }
  }
  #[cfg(not(feature = "host"))]
  pub fn into_vec(self) -> Vec<T> {
    // We create a boxed slice above, so the capacity is shrunk to `len`, so we can
    // use len for the capacity here, without leaking memory.
    unsafe { Vec::from_raw_parts(self.first as *mut T, self.len as usize, self.len as usize) }
  }
}

// On the host, this refers to data in wasm, so we don't want to free it.
#[cfg(not(feature = "host"))]
impl<T> Drop for CList<T> {
  fn drop(&mut self) {
    unsafe {
      Vec::from_raw_parts(self.first as *mut T, self.len as usize, self.len as usize);
    }
  }
}

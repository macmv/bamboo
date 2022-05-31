use std::os::raw::c_char;
#[cfg(feature = "host")]
use wasmer_types::ValueType;

/// A 32 bit pointer. Used because `WasmPtr` isn't `Copy`.
#[cfg(feature = "host")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPtr<T> {
  pub ptr:      u32,
  pub _phantom: std::marker::PhantomData<T>,
}

#[cfg(not(feature = "host"))]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPtr<T> {
  pub ptr: *const T,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPlayer {
  pub eid: i32,
}

#[cfg(feature = "host")]
unsafe impl ValueType for CPlayer {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CUUID {
  pub bytes: [u32; 4],
}

#[cfg(feature = "host")]
unsafe impl ValueType for CUUID {}

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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CCommand {
  /// The name of this command segment. For literals this is their value.
  /// Doesn't contain a NUL at the end.
  pub name:       CPtr<u8>,
  /// The length of the name.
  pub name_len:   u32,
  /// An enum.
  ///
  /// ```text
  /// 0 -> literal
  /// 1 -> argument
  /// _ -> invalid command
  /// ```
  pub node_type:  u8,
  /// This is null for `node_type` of root or literal. Doesn't contain a NUL at
  /// the end.
  pub parser:     CPtr<u8>,
  /// The length of the parser.
  pub parser_len: u32,
  /// This is a boolean, but `bool` isn't `ValueType` safe.
  pub optional:   u8,
  /// The children of this command.
  pub children:   CList,
}

#[cfg(feature = "host")]
unsafe impl ValueType for CCommand {}

// `c_void` isn't copy, so this is `u8`. This is incorrect! It
// should be a `*void`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CList {
  /// The pointer to the first element in this list.
  pub first: CPtr<u8>,
  /// The length of this list.
  pub len:   u32,
}

#[cfg(feature = "host")]
unsafe impl ValueType for CList {}

extern "C" {
  /// Logs the given message. The pointer must point to a utf8-valid
  /// nul-terminated string.
  pub fn bb_log(level: u32, message: *const u8);
  /// Logs the given message. The pointer must pointer to a utf8-valid
  /// string. There can be null bytes, but they will not terminate the
  /// message.
  pub fn bb_log_len(level: u32, message: *const u8, len: u32);

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

impl<T> CPtr<T> {
  pub fn new(ptr: *const T) -> Self {
    #[cfg(feature = "host")]
    {
      CPtr { ptr: ptr as _, _phantom: std::marker::PhantomData::default() }
    }
    #[cfg(not(feature = "host"))]
    {
      CPtr { ptr }
    }
  }
  pub fn as_ptr(&self) -> *const T { self.ptr as _ }
}

use std::fmt;
impl<T> fmt::Pointer for CPtr<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "0x{:08x}", self.as_ptr() as u32)
  }
}
impl<T> fmt::Debug for CPtr<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { fmt::Pointer::fmt(self, f) }
}

impl CList {
  pub fn new<T>(list: Vec<T>) -> Self {
    let boxed_slice = list.into_boxed_slice();
    let slice = Box::leak(boxed_slice);
    Self { first: CPtr::new(slice.as_ptr() as _), len: slice.len() as u32 }
  }
  /// # Safety
  /// - `T` must match the original `T` used to create this list.
  pub unsafe fn get<'a, T>(&'a self, index: u32) -> Option<&'a T> { Some(&*self.get_ptr(index)?) }
  /// Gets a pointer to the given element, if it is in the list.
  pub fn get_ptr<T>(&self, index: u32) -> Option<*const T> {
    if index < self.len {
      let ptr = self.first.as_ptr() as *const T;
      Some(unsafe { ptr.add(index as usize) })
    } else {
      None
    }
  }
  /// # Safety
  /// - `T` must match the original `T` used to create this list.
  pub unsafe fn into_vec<T>(self) -> Vec<T> {
    // We create a boxed slice above, so the capacity is shrunk to `len`, so we can
    // use len for the capacity here, without leaking memory.
    Vec::from_raw_parts(self.first.as_ptr() as *mut T, self.len as usize, self.len as usize)
  }
}

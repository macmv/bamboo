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
  /// Logs the given message as a debug message. The pointer must point
  /// to a nul-terminated string.
  pub fn bb_debug(message: *const u8);
  /// Logs the given message as info. The pointer must point to a
  /// nul-terminated string.
  pub fn bb_info(message: *const u8);
  /// Logs the given message as a warning. The pointer must point to
  /// a nul-terminated string.
  pub fn bb_warn(message: *const u8);
  /// Logs the given message as an error. The pointer must point to
  /// a nul-terminated string.
  pub fn bb_error(message: *const u8);

  /// Broadcasts the given chat message to all players.
  pub fn bb_broadcast(message: *const CChat);
  /// Writes the player's username into the given buffer. Returns 1
  /// if the username won't fit in the buffer, or if the pointer is
  /// invalid.
  pub fn bb_player_username(player: i32, buf: *mut u8, len: u32) -> i32;
  /// Sends the given chat message to the player.
  pub fn bb_player_send_message(player: i32, message: *const CChat);
}

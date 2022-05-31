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

extern "C" {
  /// Logs the given message. The pointer must point to a utf8-valid
  /// nul-terminated string.
  pub fn bb_log(level: u32, message: *const u8);
  /// Logs the given message. The pointer must pointer to a utf8-valid
  /// string. There can be null bytes, but they will not terminate the
  /// message.
  pub fn bb_log_len(level: u32, message: *const u8, len: u32);

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

#![deny(improper_ctypes)]

use bb_ffi_macros::{cenum, ctype};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CStr {
  #[cfg(feature = "host")]
  pub ptr: wasmer::WasmPtr<u8, wasmer::Array>,
  #[cfg(not(feature = "host"))]
  pub ptr: *mut u8,
  pub len: u32,
}

#[cfg(feature = "host")]
impl Copy for CStr {}
#[cfg(feature = "host")]
unsafe impl wasmer::ValueType for CStr {}

/// A boolean, except every bit configuration is valid. Use
/// [`as_bool`](CBool::as_bool) to convert it to a `bool`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct CBool(pub u8);

#[cfg(feature = "host")]
unsafe impl wasmer::ValueType for CBool {}

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
#[cfg_attr(not(feature = "host"), derive(Copy))]
pub struct CPos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

#[ctype]
#[derive(Debug)]
#[cfg_attr(not(feature = "host"), derive(Copy))]
pub struct CFPos {
  pub x: f64,
  pub y: f64,
  pub z: f64,
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
  pub optional:  CBool,
  /// The children of this command.
  pub children:  CList<CCommand>,
}

#[ctype]
#[derive(Debug)]
pub struct CParticle {
  /// The type of particle.
  pub ty:            CParticleType,
  /// The center of this cloud of particles.
  pub pos:           CFPos,
  /// If set, the particle will be shown to clients up to 65,000 blocks away. If
  /// not set, the particle will only render up to 256 blocks away.
  pub long_distance: CBool,
  /// The random offset for this particle cloud. This is multiplied by a random
  /// number from 0 to 1, and then added to `pos` (all on the client).
  pub offset:        CFPos,
  /// The number of particles in this cloud.
  pub count:         u32,
  /// The data for this particle. This is typically the speed of the particle,
  /// but sometimes is used for other attributes entirely.
  pub data:          f32,
}
#[ctype]
#[derive(Debug)]
pub struct CParticleType {
  pub ty:   u32,
  /// Any extra data for this particle.
  pub data: CList<u8>,
}

#[ctype]
#[derive(Debug)]
pub struct CBlockData {
  /// The kind for this data.
  pub kind:         u32,
  /// The name of this block. This is something like `grass_block`.
  pub name:         CStr,
  /// The material used to make this block. This controls things like map color,
  /// sound, what tool breaks the block, etc. Prismarine doesn't have a very
  /// good material value, so this needs to be updated to more complete data.
  pub material:     u32,
  /// Amount of time it takes to break this block.
  pub hardness:     f32,
  /// How difficult this is to break with an explosion.
  pub resistance:   f32,
  /// A list of item ids this block can drop.
  pub drops:        CList<CItemDrop>,
  /// If this is true, then clients can (at least partially) see through this
  /// block.
  pub transparent:  CBool,
  /// This is how much light this block removes. A value of 15 means it blocks
  /// all light, and a value of 0 means it blocks no light.
  pub filter_light: u8,
  /// The amount of light this block emits (0-15).
  pub emit_light:   u8,

  /// The latest version state id. This is the lowest possible state for this
  /// block. It is used to offset the state calculation for properties.
  pub state:         u32,
  /// A list of vanilla tags for this block. Plugins should be able to add tags
  /// in the future. These tags don't include `minecraft:` at the start.
  pub tags:          CList<CStr>,
  /// All the properties on this block. These are stored so that it is easy to
  /// convert a single property on a block.
  pub props:         CList<CBlockProp>,
  /// The default type. Each value is an index into that property.
  pub default_props: CList<u32>,
}

#[ctype]
#[derive(Debug)]
pub struct CItemDrop {
  pub item: CStr,
  pub min:  i32,
  pub max:  i32,
}

#[ctype]
#[derive(Debug)]
pub struct CBlockProp {
  pub name: CStr,
  pub kind: CBlockPropKind,
}

#[cenum]
pub enum CBlockPropKind {
  Bool,
  Enum(CList<CStr>),
  Int { min: u32, max: u32 },
}

#[cfg(feature = "host")]
#[repr(C)]
#[derive(Clone, Debug)]
pub struct CList<T: Copy> {
  /// The pointer to the first element in this list.
  pub first: wasmer::WasmPtr<T, wasmer::Array>,
  /// The length of this list.
  pub len:   u32,
}

#[cfg(not(feature = "host"))]
#[repr(C)]
#[derive(Clone, Debug)]
pub struct CList<T> {
  /// The pointer to the first element in this list.
  pub first: *mut T,
  /// The length of this list.
  pub len:   u32,
}

#[cfg(feature = "host")]
impl<T: Copy> Copy for CList<T> {}
#[cfg(feature = "host")]
unsafe impl<T: Copy> wasmer::ValueType for CList<T> {}

#[cenum]
pub enum CArg {
  Literal(CStr),
  Bool(CBool),
  Double(f64),
  Float(f32),
  Int(i32),
  String(CStr),
  ScoreHolder(CStr),
  BlockPos(CPos),
  Vec3(f64, f64, f64),
  Vec2(f64, f64),
  BlockState(u32),
}

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
  /// Returns the player's username. The plugin is responsible for freeing this
  /// memory.
  pub fn bb_player_username(player: *const CUUID) -> *mut CStr;
  /// Returns the current world for this player.
  pub fn bb_player_world(player: *const CUUID) -> i32;
  /// Sends the given chat message to the player.
  pub fn bb_player_send_message(player: *const CUUID, message: *const CChat);
  /// Sends the given particle to the player.
  pub fn bb_player_send_particle(player: *const CUUID, particle: *const CParticle);

  /// Sets a block in the world. Returns 1 if the block position is invalid.
  pub fn bb_world_set_block(wid: u32, pos: *const CPos, id: u32) -> i32;

  /// Returns the block data for the given kind. The plugin is responsible for
  /// freeing this memory.
  pub fn bb_block_data_for_kind(kind: u32) -> *mut CBlockData;

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

impl CBool {
  /// Creates a `CBool` of `1` if `true`, and `0` if `false`.
  pub fn new(val: bool) -> Self { CBool(if val { 1 } else { 0 }) }
  /// If the inner value is not `0`.
  pub fn as_bool(&self) -> bool { self.0 != 0 }
}

impl CStr {
  #[cfg(not(feature = "host"))]
  pub fn new(s: String) -> Self {
    let boxed_str = s.into_boxed_str();
    let s = Box::leak(boxed_str);
    CStr { ptr: s.as_ptr() as _, len: s.len() as u32 }
  }
  #[cfg(not(feature = "host"))]
  pub fn into_string(self) -> String {
    // See CList::into_vec
    if self.ptr.is_null() {
      String::new()
    } else {
      unsafe { String::from_raw_parts(self.ptr, self.len as usize, self.len as usize) }
    }
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

#[cfg(not(feature = "host"))]
impl<T> CList<T> {
  pub fn new(list: Vec<T>) -> Self {
    let boxed_slice = list.into_boxed_slice();
    let slice = Box::leak(boxed_slice);
    Self { first: slice.as_ptr() as _, len: slice.len() as u32 }
  }
  pub fn into_vec(self) -> Vec<T> {
    // Any bit layout if CList<T> is valid. Therefore, the pointer can be null. Vec
    // uses a Unique<T> internally, which can never be null. So, we just return an
    // empty list here.
    if self.first.is_null() {
      vec![]
    } else {
      // We create a boxed slice above, so the capacity is shrunk to `len`, so we can
      // use len for the capacity here, without leaking memory.
      unsafe { Vec::from_raw_parts(self.first as *mut T, self.len as usize, self.len as usize) }
    }
  }
}
#[cfg(feature = "host")]
impl<T: Copy> CList<T> {
  pub fn get_ptr(&self, index: u32) -> Option<*const T> {
    if index < self.len {
      Some(unsafe { (self.first.offset() as *const T).add(index as usize) })
    } else {
      None
    }
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

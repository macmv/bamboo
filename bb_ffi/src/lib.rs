#![deny(improper_ctypes)]

use bb_ffi_macros::{cenum, ctype};

#[repr(C)]
#[cfg_attr(feature = "host", derive(Debug, Clone))]
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
#[cfg_attr(not(feature = "host"), derive(Copy))]
pub struct CVec3 {
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
  pub parser:    CCommandParser,
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
  Int(CBlockPropKindInt),
}

#[ctype]
#[derive(Debug)]
pub struct CBlockPropKindInt {
  min: u32,
  max: u32,
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
#[derive(Debug)]
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

#[cfg(feature = "host")]
#[repr(C)]
#[derive(Clone, Debug)]
pub struct COpt<T: Copy> {
  pub present: CBool,
  pub value:   T,
}

#[cfg(not(feature = "host"))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct COpt<T> {
  pub present: CBool,
  pub value:   T,
}

#[cfg(feature = "host")]
impl<T: Copy> Copy for CList<T> {}
#[cfg(feature = "host")]
unsafe impl<T: Copy> wasmer::ValueType for CList<T> {}

#[cenum]
pub enum CCommandArg {
  Literal(CStr),
  Bool(CBool),
  Double(f64),
  Float(f32),
  Int(i32),
  String(CStr),
  ScoreHolder(CStr),
  BlockPos(CPos),
  /*
  Vec3 { x: f64, y: f64, z: f64 },
  Vec2 { x: f64, y: f64 },
  */
  BlockState(u32),
}

/// A string parsing type. Used only in [`Parser::String`].
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum StringType {
  /// Matches a single word.
  Word,
  /// Matches either a single word, or a phrase in double quotes. Quotes can be
  /// inserted in the string with `\"`.
  Quotable,
  /// Matches all remaining text in the command. Quotes are not interpreted.
  Greedy,
}

#[ctype]
#[derive(Debug)]
pub struct CCommandParserDouble {
  min: COpt<f64>,
  max: COpt<f64>,
}
#[ctype]
#[derive(Debug)]
pub struct CCommandParserFloat {
  min: COpt<f32>,
  max: COpt<f32>,
}
#[ctype]
#[derive(Debug)]
pub struct CCommandParserInt {
  min: COpt<i32>,
  max: COpt<i32>,
}
#[ctype]
#[derive(Debug)]
pub struct CCommandParserEntity {
  single:       CBool,
  only_players: CBool,
}

#[cenum]
pub enum CCommandParser {
  // Simple types:
  /// True or false.
  Bool,
  /// A double, with optional min and max values.
  Double(CCommandParserDouble),
  /// A float, with optional min and max values.
  Float(CCommandParserFloat),
  /// An int, with optional min and max values.
  Int(CCommandParserInt),
  /// A string. See [`StringType`] for details on how this is parsed.
  String(StringType),
  /// An entity. If `single` is set, then this can only match one entity (things
  /// like `@e` or `@a` are not allowed). If players is set, then only matching
  /// players (with either a username, `@p`, etc.) is allowed.
  Entity(CCommandParserEntity),
  /// A user that is on the current scoreboard. With the scoreboard system that
  /// bamboo has, this doesn't make that much sense.
  ///
  /// The bool is true if multiple targets are allowed.
  ScoreHolder(CBool),

  /// Player, online or not. Can also use a selector.
  GameProfile,
  /// location, represented as 3 numbers (which must be integers)
  BlockPos,
  /// column location, represented as 3 numbers (which must be integers)
  ColumnPos,
  /// A location, represented as 3 numbers
  Vec3,
  /// A location, represented as 2 numbers
  Vec2,
  /// A block state, optionally including NBT and state information.
  BlockState,
  /// A block, or a block tag.
  BlockPredicate,
  /// An item, optionally including NBT.
  ItemStack,
  /// An item, or an item tag.
  ItemPredicate,
  /// Chat color. One of the names from Chat#Colors, or reset.
  Color,
  /// A JSON Chat component.
  Component,
  /// A regular message, potentially including selectors.
  Message,
  /// An NBT value, parsed using JSON-NBT rules.
  Nbt,
  /// A path within an NBT value, allowing for array and member accesses.
  NbtPath,
  /// A scoreboard objective.
  Objective,
  /// A single score criterion.
  ObjectiveCriteria,
  /// A scoreboard operator.
  Operation,
  /// A particle effect
  Particle,
  /// angle, represented as 2 floats
  Rotation,
  /// A single float
  Angle,
  /// Scoreboard display position slot. list, sidebar, belowName, etc
  ScoreboardSlot,
  /// A collection of up to 3 axes.
  Swizzle,
  /// The name of a team. Parsed as an unquoted string.
  Team,
  /// A name for an inventory slot.
  ItemSlot,
  /// An Identifier.
  ResourceLocation,
  /// A potion effect.
  MobEffect,
  /// A function.
  Function,
  /// entity anchor related to the facing argument
  EntityAnchor,
  /// A range of values with a min and a max. The bool is `true` if decimals are
  /// allowed.
  Range(CBool),
  /// An integer range of values with a min and a max.
  IntRange,
  /// A floating-point range of values with a min and a max.
  FloatRange,
  /// Represents a item enchantment.
  ItemEnchantment,
  /// Represents an entity summon.
  EntitySummon,
  /// Represents a dimension.
  Dimension,
  /// Represents a UUID value.
  Uuid,
  /// Represents a partial nbt tag, usable in data modify command.
  NbtTag,
  /// Represents a full nbt tag.
  NbtCompoundTag,
  /// Represents a time duration.
  Time,

  // Forge only types:
  /// A forge mod id
  Modid,
  /// A enum class to use for suggestion. Added by Minecraft Forge.
  Enum,
}

// All functions that return a `*mut` pointer let the plugin free the memory.
// They should be used by simply passing that value into `Box::from_raw`, and
// then letting the box clean up the memory.

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
  /// Returns the player's username.
  pub fn bb_player_username(player: *const CUUID) -> *mut CStr;
  /// Returns the player's position.
  pub fn bb_player_pos(player: *const CUUID) -> *mut CFPos;
  /// Returns the player's looking direction as a unit vector.
  pub fn bb_player_look_as_vec(player: *const CUUID) -> *mut CVec3;
  /// Returns the current world for this player.
  pub fn bb_player_world(player: *const CUUID) -> i32;
  /// Sends the given chat message to the player.
  pub fn bb_player_send_message(player: *const CUUID, message: *const CChat);
  /// Sends the given particle to the player.
  pub fn bb_player_send_particle(player: *const CUUID, particle: *const CParticle);

  /// Sets a block in the world. Returns 1 if the block position is invalid.
  pub fn bb_world_set_block(wid: u32, pos: *const CPos, id: u32) -> i32;
  /// Sets a block in the world. Returns 1 if the block position is invalid.
  pub fn bb_world_players(wid: u32) -> *mut CList<CUUID>;
  /// Raycasts from the `from` position to `to`. Returns null if there is no
  /// collision.
  pub fn bb_world_raycast(from: *const CFPos, to: *const CFPos, water: CBool) -> *mut CFPos;

  /// Returns the block data for the given kind.
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
    let me = std::mem::ManuallyDrop::new(self);
    // See CList::into_vec
    if me.ptr.is_null() {
      String::new()
    } else {
      let vec = unsafe { Vec::from_raw_parts(me.ptr, me.len as usize, me.len as usize) };
      // We usually won't have to allocate. However, if we do get invalid utf8, we
      // would still like to get a string back. So, we fall back to from_utf8_lossy.
      match String::from_utf8(vec) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(&e.into_bytes()).into(),
      }
    }
  }
}

#[cfg(not(feature = "host"))]
use std::fmt;
#[cfg(not(feature = "host"))]
impl fmt::Debug for CStr {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if f.alternate() {
      f.debug_struct("CStr")
        .field("ptr", &self.ptr)
        .field("len", &self.len)
        .field(
          "str",
          &String::from_utf8_lossy(unsafe {
            std::slice::from_raw_parts(self.ptr, self.len as usize)
          }),
        )
        .finish()
    } else {
      String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(self.ptr, self.len as usize) })
        .fmt(f)
    }
  }
}

#[cfg(not(feature = "host"))]
impl Clone for CStr {
  fn clone(&self) -> Self {
    unsafe {
      let new_ptr = std::alloc::alloc(std::alloc::Layout::array::<u8>(self.len as usize).unwrap());
      std::ptr::copy(self.ptr, new_ptr, self.len as usize);
      CStr { ptr: new_ptr, len: self.len }
    }
  }
}

// On the host, this refers to data in wasm, so we don't want to free it.
#[cfg(not(feature = "host"))]
impl Drop for CStr {
  fn drop(&mut self) {
    unsafe {
      Vec::from_raw_parts(self.ptr, self.len as usize, self.len as usize);
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
    let me = std::mem::ManuallyDrop::new(self);
    // Any bit layout if CList<T> is valid. Therefore, the pointer can be null. Vec
    // uses a Unique<T> internally, which can never be null. So, we just return an
    // empty list here.
    if me.first.is_null() {
      vec![]
    } else {
      // We create a boxed slice above, so the capacity is shrunk to `len`, so we can
      // use len for the capacity here, without leaking memory.
      unsafe { Vec::from_raw_parts(me.first as *mut T, me.len as usize, me.len as usize) }
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
#[cfg(not(feature = "host"))]
impl<T: Clone> Clone for CList<T> {
  fn clone(&self) -> Self {
    unsafe {
      let new_ptr =
        std::alloc::alloc(std::alloc::Layout::array::<T>(self.len as usize).unwrap()) as *mut T;
      for i in 0..self.len as usize {
        new_ptr.add(i).write(self.first.add(i).read().clone());
      }
      CList { first: new_ptr, len: self.len }
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

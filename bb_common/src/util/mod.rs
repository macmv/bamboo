pub mod chat;
mod pool;

pub use chat::Chat;
pub use pool::ThreadPool;

mod buffer;
mod item;

use crate::math::Pos;
use bb_macros::Transfer;
#[cfg(feature = "host")]
use rand::{rngs::OsRng, RngCore};
use serde::de::{self, Deserialize, Deserializer, Unexpected, Visitor};
use std::{error::Error, fmt, num::ParseIntError, str::FromStr};

pub use buffer::{Buffer, BufferError, BufferErrorKind, Mode};
pub use item::Item;

pub use num_cpus::get as num_cpus;

pub fn serialize_varint(v: i32) -> Vec<u8> {
  // Need to work with u32, as >> acts differently on i32 vs u32.
  let mut val = v as u32;
  let mut out = vec![];
  for _ in 0..5 {
    let mut b: u8 = val as u8 & 0b01111111;
    val >>= 7;
    if val != 0 {
      b |= 0b10000000;
    }
    out.push(b);
    if val == 0 {
      break;
    }
  }
  out
}

pub fn read_varint(buf: &[u8]) -> (i32, isize) {
  let mut res: i32 = 0;
  let mut total_read: isize = 0;
  for i in 0..5 {
    if i >= buf.len() {
      // Incomplete varint
      return (0, 0);
    }
    let read = buf[i];
    if i == 4 && read & 0b10000000 != 0 {
      // Invalid varint (read < 0 means invalid varint)
      return (0, -1);
    }

    let v = read & 0b01111111;
    res |= (v as i32) << (7 * i);

    if read & 0b10000000 == 0 {
      // Done reading bytes, so we set total read
      total_read = i as isize + 1;
      break;
    }
  }
  (res, total_read)
}

use crate::nbt::NBT;
use bb_transfer::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};

impl MessageRead<'_> for UUID {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(UUID::from_le_bytes(m.read_bytes()?.try_into().unwrap()))
  }
}
impl MessageWrite for UUID {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_bytes(&self.as_le_bytes())?;
    Ok(())
  }
}

#[cfg(feature = "host")]
impl MessageRead<'_> for NBT {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    // TODO: ParseError into ReadError
    Ok(NBT::deserialize(m.read_bytes()?.to_vec()).unwrap())
  }
}
#[cfg(not(feature = "host"))]
impl MessageRead<'_> for NBT {
  fn read(_: &mut MessageReader) -> Result<Self, ReadError> { panic!("cannot read NBT in plugin") }
}
impl MessageWrite for NBT {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_bytes(&self.serialize())
  }
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
  #[id = 0]
  Main,
  #[id = 1]
  Off,
}

impl Default for Hand {
  fn default() -> Self { Hand::Main }
}

impl Hand {
  pub fn id(&self) -> u8 {
    match self {
      Self::Main => 0,
      Self::Off => 1,
    }
  }

  pub fn from_id(id: u8) -> Hand {
    match id {
      0 => Self::Main,
      1 => Self::Off,
      _ => panic!("invalid hand: {}", id),
    }
  }
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GameMode {
  #[id = 0]
  Survival,
  #[id = 1]
  Creative,
  #[id = 2]
  Adventure,
  #[id = 3]
  Spectator,
}

impl Default for GameMode {
  fn default() -> Self { GameMode::Survival }
}

impl GameMode {
  pub fn id(&self) -> u8 {
    match self {
      Self::Survival => 0,
      Self::Creative => 1,
      Self::Adventure => 2,
      Self::Spectator => 3,
    }
  }

  pub fn from_id(id: u8) -> Self {
    match id {
      0 => Self::Survival,
      1 => Self::Creative,
      2 => Self::Adventure,
      3 => Self::Spectator,
      _ => panic!("invalid gamemode: {}", id),
    }
  }
}

#[derive(Debug)]
pub struct InvalidGameMode(String);

impl fmt::Display for InvalidGameMode {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid game mode: {}", self.0)
  }
}

impl Error for InvalidGameMode {}

impl FromStr for GameMode {
  type Err = InvalidGameMode;

  fn from_str(s: &str) -> Result<Self, InvalidGameMode> {
    Ok(match s {
      "survival" => GameMode::Survival,
      "creative" => GameMode::Creative,
      "adventure" => GameMode::Adventure,
      "spectator" => GameMode::Spectator,
      _ => return Err(InvalidGameMode(s.into())),
    })
  }
}

#[derive(Transfer, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
  #[id = 0]
  Bottom,
  #[id = 1]
  Top,
  #[id = 2]
  North,
  #[id = 3]
  South,
  #[id = 4]
  West,
  #[id = 5]
  East,
}

impl Default for Face {
  fn default() -> Self { Face::Bottom }
}

impl Face {
  pub fn id(&self) -> u8 {
    match self {
      Self::Bottom => 0,
      Self::Top => 1,
      Self::North => 2,
      Self::South => 3,
      Self::West => 4,
      Self::East => 5,
    }
  }

  pub fn as_dir(&self) -> Pos {
    match self {
      Self::Bottom => Pos::new(0, -1, 0),
      Self::Top => Pos::new(0, 1, 0),
      Self::North => Pos::new(0, 0, -1),
      Self::South => Pos::new(0, 0, 1),
      Self::West => Pos::new(-1, 0, 0),
      Self::East => Pos::new(1, 0, 0),
    }
  }

  pub fn from_id(id: u8) -> Face {
    match id {
      0 => Self::Bottom,
      1 => Self::Top,
      2 => Self::North,
      3 => Self::South,
      4 => Self::West,
      5 => Self::East,
      _ => panic!("invalid block face: {}", id),
    }
  }

  pub fn as_str(&self) -> &str {
    match self {
      Self::Bottom => "BOTTOM",
      Self::Top => "TOP",
      Self::North => "NORTH",
      Self::South => "SOUTH",
      Self::West => "WEST",
      Self::East => "EAST",
    }
  }
}
impl From<&str> for Face {
  fn from(s: &str) -> Face {
    match s {
      "BOTTOM" => Self::Bottom,
      "TOP" => Self::Top,
      "NORTH" => Self::North,
      "SOUTH" => Self::South,
      "WEST" => Self::West,
      "EAST" => Self::East,
      _ => Self::North,
    }
  }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct UUID(u128);

impl Default for UUID {
  fn default() -> UUID { UUID::from_u128(0) }
}

#[derive(Debug)]
pub enum UUIDParseError {
  Int(ParseIntError),
  Length(usize),
}

impl fmt::Display for UUIDParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "error while parsing uuid: {}",
      match self {
        Self::Int(e) => format!("int parsing error: {}", e),
        Self::Length(len) => format!("invalid length: {}", len),
      }
    )
  }
}

impl Error for UUIDParseError {}

impl UUID {
  /// Generates a random UUID. This uses rand::OsRng, so it will be secure.
  #[cfg(feature = "host")]
  pub fn random() -> Self {
    let mut arr = [0; 16];
    OsRng.fill_bytes(&mut arr);
    UUID::from_be_bytes(arr)
  }
  pub fn from_le_bytes(v: [u8; 16]) -> Self { UUID(u128::from_le_bytes(v)) }
  pub fn from_be_bytes(v: [u8; 16]) -> Self { UUID(u128::from_be_bytes(v)) }
  pub fn from_u128(v: u128) -> Self { UUID(v) }
  /// Parses the string as a uuid with dashes in between. This is the same
  /// format returned from [`as_dashed_str`](Self::as_dashed_str).
  pub fn from_dashed_str(s: &str) -> Result<Self, UUIDParseError> {
    if s.len() != 36 {
      return Err(UUIDParseError::Length(s.len()));
    }
    Self::from_str(&s.split('-').collect::<Vec<&str>>().join(""))
  }
  /// Returns the uuid represented as a hex string, with no dashes or other
  /// characters.
  pub fn as_str(&self) -> String { format!("{:x}", self.0) }
  /// Returns the uuid represented as a string with dashes. This is used
  /// sometimes when refering to player in json, and is a useful function to
  /// have.
  pub fn as_dashed_str(&self) -> String {
    format!(
      "{:x}-{:x}-{:x}-{:x}-{:x}",
      //          11111111222233334444555555555555
      (self.0 & 0xffffffff000000000000000000000000) >> (24 * 4), // 4 bits per digit
      (self.0 & 0x00000000ffff00000000000000000000) >> (20 * 4),
      (self.0 & 0x000000000000ffff0000000000000000) >> (16 * 4),
      (self.0 & 0x0000000000000000ffff000000000000) >> (12 * 4),
      (self.0 & 0x00000000000000000000ffffffffffff),
    )
  }
  /// Returns the underlying `u128`. For packets, you probably want
  /// [`as_be_bytes`](Self::as_be_bytes). For json, you probably want
  /// [`as_str`](Self::as_str) or [`as_dashed_str`](Self::as_dashed_str).
  pub fn as_u128(&self) -> u128 { self.0 }
  /// Returns the little-endian representation of the underlying `u128`. This is
  /// the byte order that the Minecraft Bedrock Edition uses in its packet
  /// protocol.
  pub fn as_le_bytes(&self) -> [u8; 16] { self.0.to_le_bytes() }
  /// Returns the big-endian representation of the underlying `u128`. This is
  /// the byte order that the Minecraft Java Edition uses in its packet
  /// protocol.
  pub fn as_be_bytes(&self) -> [u8; 16] { self.0.to_be_bytes() }
}

impl FromStr for UUID {
  type Err = UUIDParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.len() != 32 {
      return Err(UUIDParseError::Length(s.len()));
    }
    match u128::from_str_radix(s, 16) {
      Ok(v) => Ok(Self::from_u128(v)),
      Err(e) => Err(UUIDParseError::Int(e)),
    }
  }
}

impl<'de> Deserialize<'de> for UUID {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct Inner;
    impl<'de> Visitor<'de> for Inner {
      type Value = UUID;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a UUID")
      }

      fn visit_str<E>(self, value: &str) -> Result<UUID, E>
      where
        E: de::Error,
      {
        UUID::from_str(value).map_err(|_| de::Error::invalid_value(Unexpected::Str(value), &self))
      }
    }
    deserializer.deserialize_str(Inner)
  }
}

#[derive(Transfer, Debug, Clone)]
pub struct JoinInfo {
  #[must_exist]
  pub mode:     JoinMode,
  #[must_exist]
  pub username: String,
  #[must_exist]
  pub uuid:     UUID,
  #[must_exist]
  pub ver:      u32,
}

#[derive(Transfer, Debug, Clone)]
pub enum JoinMode {
  #[id = 0]
  New,
  #[id = 1]
  Switch,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn uuid_dashed_str() {
    let uuid = UUID::from_u128(0x11111111222233334444555555555555);
    assert_eq!(uuid.as_dashed_str(), "11111111-2222-3333-4444-555555555555");
  }
}

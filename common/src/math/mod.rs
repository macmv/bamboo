mod chunk_pos;
mod fpos;
mod point_grid;
mod pos;
mod rng;
mod voronoi;

pub use chunk_pos::ChunkPos;
pub use fpos::{FPos, FPosError};
pub use point_grid::PointGrid;
pub use pos::{Pos, PosError};
pub use rng::WyhashRng;
pub use voronoi::Voronoi;

use serde::de::{self, Deserialize, Deserializer, Unexpected, Visitor};
use std::{convert::TryInto, error::Error, fmt, num::ParseIntError, str::FromStr};

use crate::proto;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct UUID(u128);

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
  pub fn from_bytes(v: [u8; 16]) -> Self {
    UUID(u128::from_be_bytes(v))
  }
  pub fn from_u128(v: u128) -> Self {
    UUID(v)
  }
  pub fn from_proto(v: proto::Uuid) -> Self {
    Self(u128::from_be_bytes(v.be_data.try_into().unwrap()))
  }
  /// Parses the string as a uuid with dashes in between. This is the same
  /// format returned from [`as_dashed_str`](Self::as_dashed_str).
  pub fn from_dashed_str(s: &str) -> Result<Self, UUIDParseError> {
    if s.len() != 36 {
      return Err(UUIDParseError::Length(s.len()));
    }
    Self::from_str(&s.split('-').collect::<Vec<&str>>().join(""))
  }
  pub fn as_proto(&self) -> proto::Uuid {
    proto::Uuid { be_data: self.as_be_bytes().to_vec() }
  }
  /// Returns the uuid represented as a hex string, with no dashes or other
  /// characters.
  pub fn as_str(&self) -> String {
    format!("{}", self.0)
  }
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
  pub fn as_u128(&self) -> u128 {
    self.0
  }
  /// Returns the little-endian representation of the underlying `u128`. This is
  /// the byte order that the Minecraft Bedrock Edition uses in its packet
  /// protocol.
  pub fn as_le_bytes(&self) -> [u8; 16] {
    self.0.to_le_bytes()
  }
  /// Returns the big-endian representation of the underlying `u128`. This is
  /// the byte order that the Minecraft Java Edition uses in its packet
  /// protocol.
  pub fn as_be_bytes(&self) -> [u8; 16] {
    self.0.to_be_bytes()
  }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn uuid_dashed_str() {
    let uuid = UUID::from_u128(0x11111111222233334444555555555555);
    assert_eq!(uuid.as_dashed_str(), "11111111-2222-3333-4444-555555555555");
  }
}

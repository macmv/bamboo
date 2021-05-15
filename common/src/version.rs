use num_derive::{FromPrimitive, ToPrimitive};
use strum_macros::EnumString;

/// A list of all supported block versions. This is mostly the same as all major
/// versions of the game. Any time the game gets new blocks, there is a new
/// version added to this enum.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, FromPrimitive, ToPrimitive)]
pub enum BlockVersion {
  Invalid,
  V1_8,
  V1_9,
  V1_10,
  V1_11,
  V1_12,
  V1_13,
  V1_14,
  V1_15,
  V1_16,
}

impl BlockVersion {
  /// Returns the latest version. This is the version that block ids are stored
  /// in.
  pub fn latest() -> Self {
    Self::V1_16
  }
  /// Returns the protocol version for this block version. This will always
  /// return the latest version that uses this block version.
  pub fn protocol(&self) -> ProtocolVersion {
    // This should always be exaustive, so that new versions don't get missed.
    match self {
      Self::Invalid => ProtocolVersion::Invalid,
      Self::V1_8 => ProtocolVersion::V1_8,
      Self::V1_9 => ProtocolVersion::V1_9_4,
      Self::V1_10 => ProtocolVersion::V1_10,
      Self::V1_11 => ProtocolVersion::V1_11_2,
      Self::V1_12 => ProtocolVersion::V1_12_2,
      Self::V1_13 => ProtocolVersion::V1_13_2,
      Self::V1_14 => ProtocolVersion::V1_14_4,
      Self::V1_15 => ProtocolVersion::V1_15_2,
      Self::V1_16 => ProtocolVersion::V1_16_5,
    }
  }

  /// Returns the protocol version from the given index. 0 -> 1.8, 1 -> 1.9,
  /// etc.
  pub fn from_index(v: u32) -> Self {
    match num::FromPrimitive::from_u32(v) {
      Some(v) => v,
      None => Self::Invalid,
    }
  }

  pub fn to_index(self) -> u32 {
    num::ToPrimitive::to_u32(&self).unwrap_or(0)
  }
}

/// A list of all protocol versions. This is mostly inclusive to what this
/// server supports. I do not plan to add support for anything pre-1.8.
/// Currently, 1.9 - 1.11 is not worth my time, so I probably will never support
/// those. However, 1.9 - 1.11 is always an option, if anyone wants to implement
/// it.
///
/// If any protocol versions collide, there are two rules to follow: If it is a
/// major version, keep that one. Otherwise, use the highest major version. This
/// means that things like 1.10.1 and 1.10.2 are removed in favor of 1.10. It
/// also means that 1.9.3 is removed in favor of 1.9.4.
///
/// This will always be non exhaustive, as there will always be new versions
/// added to the game.
#[non_exhaustive]
#[derive(
  Clone, Copy, FromPrimitive, ToPrimitive, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, EnumString,
)]
pub enum ProtocolVersion {
  Invalid = 0,

  V1_8    = 47,

  V1_9    = 107,
  V1_9_2  = 109,
  V1_9_4  = 110,

  V1_10   = 210,

  V1_11   = 315,
  V1_11_2 = 316,

  V1_12   = 335,
  V1_12_1 = 338,
  V1_12_2 = 340,

  V1_13   = 393,
  V1_13_1 = 401,
  V1_13_2 = 404,

  V1_14   = 477,
  V1_14_1 = 480,
  V1_14_2 = 485,
  V1_14_3 = 490,
  V1_14_4 = 498,

  V1_15   = 573,
  V1_15_1 = 575,
  V1_15_2 = 578,

  V1_16   = 735,
  V1_16_1 = 736,
  V1_16_2 = 751,
  V1_16_3 = 753,
  V1_16_5 = 754,
}

impl ProtocolVersion {
  /// Creates a new protocol version from the given id. If the version is
  /// invalid, then this returns `VInvalid`.
  pub fn from(v: i32) -> Self {
    match num::FromPrimitive::from_i32(v) {
      Some(v) => v,
      None => Self::Invalid,
    }
  }
  /// Converts the given string to a protocol version. This string should be in
  /// the same format as the enums. That is, V1_12_2 would get
  /// `ProtocolVersion::V1_12_2`.
  pub fn from_str(s: &str) -> Self {
    match s.parse() {
      Ok(v) => v,
      Err(_) => Self::Invalid,
    }
  }
  /// Returns the protocol id. This is the version that is sent to the server
  /// from the client. If this is 0, then this is an invalid protocol.
  pub fn id(&self) -> u32 {
    num::ToPrimitive::to_u32(self).unwrap_or(0)
  }
  /// Returns the block version that this protocol version uses.
  pub fn block(&self) -> BlockVersion {
    match self {
      // Should always be exaustive, so that new versions aren't missed.
      Self::Invalid => BlockVersion::Invalid,
      Self::V1_8 => BlockVersion::V1_8,
      Self::V1_9 => BlockVersion::V1_9,
      Self::V1_9_2 => BlockVersion::V1_9,
      Self::V1_9_4 => BlockVersion::V1_9,
      Self::V1_10 => BlockVersion::V1_10,
      Self::V1_11 => BlockVersion::V1_11,
      Self::V1_11_2 => BlockVersion::V1_11,
      Self::V1_12 => BlockVersion::V1_12,
      Self::V1_12_1 => BlockVersion::V1_12,
      Self::V1_12_2 => BlockVersion::V1_12,
      Self::V1_13 => BlockVersion::V1_13,
      Self::V1_13_1 => BlockVersion::V1_13,
      Self::V1_13_2 => BlockVersion::V1_13,
      Self::V1_14 => BlockVersion::V1_14,
      Self::V1_14_1 => BlockVersion::V1_14,
      Self::V1_14_2 => BlockVersion::V1_14,
      Self::V1_14_3 => BlockVersion::V1_14,
      Self::V1_14_4 => BlockVersion::V1_14,
      Self::V1_15 => BlockVersion::V1_15,
      Self::V1_15_1 => BlockVersion::V1_15,
      Self::V1_15_2 => BlockVersion::V1_15,
      Self::V1_16 => BlockVersion::V1_16,
      Self::V1_16_1 => BlockVersion::V1_16,
      Self::V1_16_2 => BlockVersion::V1_16,
      Self::V1_16_3 => BlockVersion::V1_16,
      Self::V1_16_5 => BlockVersion::V1_16,
    }
  }
}

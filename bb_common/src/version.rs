use num_derive::{FromPrimitive, ToPrimitive};
use std::fmt;
use strum_macros::EnumString;

macro_rules! ignore {
  ($v: expr, $e: expr) => {
    $v
  };
}

macro_rules! block_version {
  [$([$v: ident, $pv: ident]),*,] => {

    /// A list of all supported block versions. This is mostly the same as all major
    /// versions of the game. Any time the game gets new blocks, there is a new
    /// version added to this enum.
    #[non_exhaustive]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, FromPrimitive, ToPrimitive)]
    pub enum BlockVersion {
      $(
        $v,
      )*
      Invalid,
    }

    impl BlockVersion {
      /// Returns the protocol version for this block version. This will always
      /// return the latest version that uses this block version.
      pub fn protocol(&self) -> ProtocolVersion {
        // This should always be exhaustive, so that new versions don't get missed.
        match self {
          Self::Invalid => ProtocolVersion::Invalid,
          $(
            Self::$v => ProtocolVersion::$pv,
          )*
        }
      }
      /// This returns the total number of block versions.
      pub fn len() -> u32 {
        0 $(+ ignore!(1, $v))*
      }
    }
  }
}

block_version![
  [V1_8, V1_8],
  [V1_9, V1_9_4],
  [V1_10, V1_10_2],
  [V1_11, V1_11_2],
  [V1_12, V1_12_2],
  // [V1_13, V1_13_2],
  [V1_14, V1_14_4],
  [V1_15, V1_15_2],
  [V1_16, V1_16_5],
  [V1_17, V1_17_1],
  [V1_18, V1_18_2],
  [V1_19, V1_19_4],
  [V1_20, V1_20],
];

impl BlockVersion {
  /// Returns the latest version. This is the version that block ids are stored
  /// in.
  pub const fn latest() -> Self { Self::V1_20 }

  /// Returns the protocol version from the given index. 0 -> latest, 1 -> one
  /// before latest, etc.
  pub fn from_index_rev(v: u32) -> Self {
    match num::FromPrimitive::from_u32(Self::len() - v) {
      Some(v) => v,
      None => Self::Invalid,
    }
  }

  /// Returns the given index of this block version. Latest -> 0, the version
  /// before latest -> 1, etc.
  pub fn to_index_rev(self) -> u32 { Self::len() - num::ToPrimitive::to_u32(&self).unwrap_or(0) }

  /// Returns the protocol version from the given index. 0 -> 1.8, 1 -> 1.9,
  /// etc.
  pub fn from_index(v: u32) -> Self {
    match num::FromPrimitive::from_u32(v) {
      Some(v) => v,
      None => Self::Invalid,
    }
  }

  /// Returns the given index of this block version. 1.8 -> 0, 1.9 -> 1, etc.
  pub fn to_index(self) -> u32 { num::ToPrimitive::to_u32(&self).unwrap_or(0) }
}

/// A list of all protocol versions. This is mostly inclusive to what this
/// server supports. I do not plan to add support for anything pre-1.8.
/// Currently, 1.9 - 1.11 is not worth my time, so I probably will never support
/// those. However, 1.9 - 1.11 is always an option, if anyone wants to implement
/// it.
///
/// If any protocol versions collide, there is always a simple rule to follow:
/// use the highest minor version. This means that things like 1.10 and 1.10.1
/// are removed in favor of 1.10.2.
///
/// This will always be non exhaustive, as there will always be new versions
/// added to the game.
///
/// NOTE: Remember to update the versions in `bb_data` as well!
#[non_exhaustive]
#[bb_macros::protocol_version]
#[derive(
  Clone, Copy, FromPrimitive, ToPrimitive, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, EnumString,
)]
pub enum ProtocolVersion {
  V1_8    = 47,

  V1_9    = 107,
  V1_9_2  = 109,
  V1_9_4  = 110,

  V1_10_2 = 210,

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

  V1_17   = 755,
  V1_17_1 = 756,

  V1_18   = 757,
  V1_18_2 = 758,

  V1_19   = 759,
  V1_19_2 = 760,
  V1_19_3 = 761,
  V1_19_4 = 762,

  V1_20   = 763,
}

impl ProtocolVersion {
  /// Returns the latest protocol version.
  pub const fn latest() -> Self { Self::V1_20 }

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
  /// `ProtocolVersion::V1_12_2`. This will return [`Self::Invalid`]
  /// if the string could not be parsed.
  pub fn parse_str(s: &str) -> Self {
    match s.parse() {
      Ok(v) => v,
      Err(_) => Self::Invalid,
    }
  }

  /// Returns the protocol id. This is the version that is sent to the server
  /// from the client. If this is 0, then this is an invalid protocol.
  pub fn id(&self) -> u32 { num::ToPrimitive::to_u32(self).unwrap_or(0) }
  /// Returns the block version that this protocol version uses.
  pub fn block(&self) -> BlockVersion {
    match self {
      // Should always be exhaustive, so that new versions aren't missed.
      Self::Invalid => BlockVersion::Invalid,
      Self::V1_8 => BlockVersion::V1_8,
      Self::V1_9 => BlockVersion::V1_9,
      Self::V1_9_2 => BlockVersion::V1_9,
      Self::V1_9_4 => BlockVersion::V1_9,
      Self::V1_10_2 => BlockVersion::V1_10,
      Self::V1_11 => BlockVersion::V1_11,
      Self::V1_11_2 => BlockVersion::V1_11,
      Self::V1_12 => BlockVersion::V1_12,
      Self::V1_12_1 => BlockVersion::V1_12,
      Self::V1_12_2 => BlockVersion::V1_12,
      // Self::V1_13 => BlockVersion::V1_13,
      // Self::V1_13_1 => BlockVersion::V1_13,
      // Self::V1_13_2 => BlockVersion::V1_13,
      Self::V1_13 => unimplemented!("1.13 has been removed. There are no plans to re-add it."),
      Self::V1_13_1 => unimplemented!("1.13 has been removed. There are no plans to re-add it."),
      Self::V1_13_2 => unimplemented!("1.13 has been removed. There are no plans to re-add it."),
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
      Self::V1_17 => BlockVersion::V1_17,
      Self::V1_17_1 => BlockVersion::V1_17,
      Self::V1_18 => BlockVersion::V1_18,
      Self::V1_18_2 => BlockVersion::V1_18,
      Self::V1_19 => BlockVersion::V1_19,
      Self::V1_19_2 => BlockVersion::V1_19,
      Self::V1_19_3 => BlockVersion::V1_19,
      Self::V1_19_4 => BlockVersion::V1_19,
      Self::V1_20 => BlockVersion::V1_20,
    }
  }
}

impl fmt::Display for ProtocolVersion {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if *self == Self::Invalid {
      write!(f, "Invalid version")
    } else if self.min().unwrap() == 0 {
      write!(f, "1.{}", self.maj().unwrap())
    } else {
      write!(f, "1.{}.{}", self.maj().unwrap(), self.min().unwrap())
    }
  }
}

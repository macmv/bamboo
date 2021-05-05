use num_derive::{FromPrimitive, ToPrimitive};

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
#[derive(Clone, Copy, FromPrimitive, ToPrimitive, Debug, PartialEq, Eq, Hash)]
pub enum ProtocolVersion {
  VInvalid = 0,

  V1_8 = 47,

  V1_9 = 107,
  V1_9_2 = 109,
  V1_9_4 = 110,

  V1_10 = 210,

  V1_11 = 315,
  V1_11_2 = 316,

  V1_12 = 335,
  V1_12_1 = 338,
  V1_12_2 = 340,

  V1_13 = 393,
  V1_13_1 = 401,
  V1_13_2 = 404,

  V1_14 = 477,
  V1_14_1 = 480,
  V1_14_2 = 485,
  V1_14_3 = 490,
  V1_14_4 = 498,

  V1_15 = 573,
  V1_15_1 = 575,
  V1_15_2 = 578,

  V1_16 = 735,
  V1_16_1 = 736,
  V1_16_2 = 751,
  V1_16_3 = 753,
  V1_16_5 = 754,
}

impl ProtocolVersion {
  /// Creates a new protocol version from the given id. If the version is
  /// invalid, then this returns `VInvalid`.
  pub fn from(v: u32) -> Self {
    match num::FromPrimitive::from_u32(v) {
      Some(v) => v,
      None => Self::VInvalid,
    }
  }
  /// Returns the protocol id. This is the version that is sent to the server
  /// from the client. If this is 0, then this is an invalid protocol.
  pub fn id(&self) -> u32 {
    match num::ToPrimitive::to_u32(self) {
      Some(v) => v,
      None => 0,
    }
  }
}

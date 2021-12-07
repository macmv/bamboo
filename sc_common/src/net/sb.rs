use super::VersionConverter;
use crate::version::ProtocolVersion;
use sc_generated::net::sb::Packet as GPacket;
use std::{error::Error, fmt};

#[derive(Debug, Clone, sc_macros::Packet)]
pub enum Packet {
  Chat {
    msg: String,
  },
  PlayerOnGround {
    on_ground: bool,
  },
  PlayerLook {
    yaw:       f32,
    pitch:     f32,
    on_ground: bool,
  },
  PlayerPos {
    x:         f64,
    y:         f64,
    z:         f64,
    on_ground: bool,
  },
  PlayerPosLook {
    x:         f64,
    y:         f64,
    z:         f64,
    yaw:       f32,
    pitch:     f32,
    on_ground: bool,
  },
}

#[derive(Debug, Clone)]
pub struct ReadError {
  packet: GPacket,
  kind:   ReadErrorKind,
}

#[derive(Debug, Clone)]
pub enum ReadErrorKind {
  UnknownPacket,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.kind {
      ReadErrorKind::UnknownPacket => write!(f, "unknown packet {:?}", self.packet),
    }
  }
}

impl Error for ReadError {}

impl Packet {
  pub fn from_tcp(
    p: GPacket,
    _ver: ProtocolVersion,
    _conv: &impl VersionConverter,
  ) -> Result<Self, ReadError> {
    Ok(match p {
      GPacket::PlayerV8 { on_ground, .. } => Packet::PlayerOnGround { on_ground },
      // TODO: The `super` call in the player movement packets is not parsed correctly.
      GPacket::PlayerLookV8 { yaw, pitch, .. } | GPacket::PlayerRotationV9 { yaw, pitch, .. } => {
        Packet::PlayerLook { yaw, pitch, on_ground: false }
      }
      GPacket::PlayerPosLookV8 { x, y, z, yaw, pitch, .. }
      | GPacket::PlayerPositionRotationV9 { x, y, z, yaw, pitch, .. } => {
        Packet::PlayerPosLook { x, y, z, yaw, pitch, on_ground: false }
      }
      GPacket::PlayerPositionV8 { x, y, z, .. } => Packet::PlayerPos { x, y, z, on_ground: false },
      _ => return Err(ReadError { packet: p, kind: ReadErrorKind::UnknownPacket }),
    })
  }
}

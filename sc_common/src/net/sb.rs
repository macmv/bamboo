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
pub enum ReadError {
  UnknownPacket,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::UnknownPacket => write!(f, "unknown packet"),
    }
  }
}

impl Error for ReadError {}

impl Packet {
  pub fn from_tcp(p: GPacket, ver: ProtocolVersion) -> Result<Self, ReadError> {
    Ok(match p {
      GPacket::PlayerV8 { on_ground, .. } => Packet::PlayerOnGround { on_ground },
      _ => return Err(ReadError::UnknownPacket),
    })
  }
}

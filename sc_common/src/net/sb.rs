use crate::version::ProtocolVersion;
use sc_generated::net::sb::Packet as GPacket;
use std::{error::Error, fmt};

#[derive(Debug, Clone, sc_macros::Packet)]
pub enum Packet {
  PlayerMove {},
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
      GPacket::PlayerV8 { .. } => Packet::PlayerMove {},
      _ => return Err(ReadError::UnknownPacket),
    })
  }
}

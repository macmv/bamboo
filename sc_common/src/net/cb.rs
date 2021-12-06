use crate::version::ProtocolVersion;
use sc_generated::net::cb::Packet as GPacket;
use std::{error::Error, fmt};

#[derive(Debug, Clone, sc_macros::Packet)]
pub enum Packet {
  Chunk { x: i32, z: i32, palette: Vec<u32>, blocks: Vec<u32> },
  KeepAlive { id: u32 },
}

#[derive(Debug, Clone)]
pub enum WriteError {
  InvalidVer,
}

impl fmt::Display for WriteError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::InvalidVer => write!(f, "invalid version"),
    }
  }
}

impl Error for WriteError {}

impl Packet {
  pub fn to_tcp(&self, ver: ProtocolVersion) -> Result<GPacket, WriteError> {
    Ok(match self {
      // Packet::Chunk { .. } => GPacket::ChunkDataV8 {},
      _ => todo!(),
    })
  }
}

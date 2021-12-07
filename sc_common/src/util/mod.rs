pub mod chat;
mod pool;

pub use chat::Chat;
pub use pool::ThreadPool;

pub use sc_generated::util::{nbt, read_varint, serialize_varint, Buffer, BufferError, UUID};

use sc_transfer::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GameMode {
  Survival,
  Creative,
  Adventure,
  Spectator,
}

impl MessageRead for GameMode {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { Ok(Self::from_id(m.read_u8()?)) }
}

impl MessageWrite for GameMode {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> { m.write_u8(self.id()) }
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

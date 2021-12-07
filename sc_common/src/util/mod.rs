pub mod chat;
mod pool;

pub use chat::Chat;
pub use pool::ThreadPool;

pub use sc_generated::util::{nbt, read_varint, serialize_varint, Buffer, BufferError, UUID};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, sc_macros::Transfer)]
pub enum GameMode {
  Survival,
  Creative,
  Adventure,
  Spectator,
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

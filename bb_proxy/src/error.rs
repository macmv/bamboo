use crate::gnet::{cb, sb};
use bb_common::{util::BufferError, version::ProtocolVersion};
use std::{fmt, io, net::AddrParseError};

#[derive(Debug)]
pub enum Error {
  Buffer(BufferError),
  IO(io::Error),
  Addr(AddrParseError),
  TransferRead(bb_transfer::ReadError),
  TransferWrite(bb_transfer::WriteError),
  UnknownCB(Box<cb::Packet>),
  UnknownSB(Box<sb::Packet>),
  ParseError {
    msg: &'static str,
    err: Box<dyn std::error::Error>,
    id:  i32,
    ver: ProtocolVersion,
    pos: usize,
    sb:  bool,
  },
  BungeecordError {
    msg: &'static str,
  },
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::Buffer(e) => write!(f, "{e}"),
      Self::IO(e) => write!(f, "{e}"),
      Self::Addr(e) => write!(f, "invalid address: {e}"),
      Self::TransferRead(e) => write!(f, "while reading from server: {e}"),
      Self::TransferWrite(e) => write!(f, "while writing to server: {e}"),
      Self::UnknownCB(p) => write!(f, "unknown clientbound packet {p:?}"),
      Self::UnknownSB(p) => write!(f, "unknown serverbound packet {p:?}"),
      Self::ParseError { msg, err, id, ver, pos, sb } => {
        write!(
          f,
          "parse error for {} packet {id:#x} (name: {}) on version {ver:?} (at byte {pos:#x}): while in {msg}, got error: {err}",
          if *sb { "serverbound" } else { "clientbound" },
          if *sb { sb::tcp_name(*id, *ver) } else { cb::tcp_name(*id, *ver) },
        )
      }
      Self::BungeecordError { msg } => {
        write!(f, "bungeecord error {msg}",)
      }
    }
  }
}

impl std::error::Error for Error {}

impl From<BufferError> for Error {
  fn from(e: BufferError) -> Self { Error::Buffer(e) }
}
impl From<io::Error> for Error {
  fn from(e: io::Error) -> Self { Error::IO(e) }
}
impl From<AddrParseError> for Error {
  fn from(e: AddrParseError) -> Self { Error::Addr(e) }
}
impl From<bb_transfer::ReadError> for Error {
  fn from(e: bb_transfer::ReadError) -> Self { Error::TransferRead(e) }
}
impl From<bb_transfer::WriteError> for Error {
  fn from(e: bb_transfer::WriteError) -> Self { Error::TransferWrite(e) }
}

impl Error {
  pub fn io_kind(&self) -> Option<io::ErrorKind> {
    match self {
      Self::IO(e) => Some(e.kind()),
      _ => None,
    }
  }
}

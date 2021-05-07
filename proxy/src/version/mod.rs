mod cb;
mod sb;

use common::{
  net::{cb::Packet as CbPacket, sb::Packet as SbPacket},
  version::ProtocolVersion,
};
use std::io;

use crate::packet::Packet;

pub struct Generator {
  cb: cb::Generator,
  sb: sb::Generator,
}

impl Generator {
  pub fn new() -> Generator {
    Generator { cb: cb::Generator::new(), sb: sb::Generator::new() }
  }
  pub fn clientbound(&self, v: ProtocolVersion, p: CbPacket) -> io::Result<Packet> {
    self.cb.convert(v, p)
  }
  pub fn serverbound(&self, v: ProtocolVersion, p: Packet) -> io::Result<SbPacket> {
    self.sb.convert(v, p)
  }
}

mod cb;
mod sb;

use common::{net::cb::Packet as CbPacket, version::ProtocolVersion};

use crate::packet::Packet;

pub struct Generator {
  cb: cb::Generator,
  // sb: sb::Generator,
}

impl Generator {
  pub fn new() -> Generator {
    Generator { cb: cb::Generator::new() }
  }
  pub fn clientbound(&self, v: ProtocolVersion, p: CbPacket) -> Packet {
    self.cb.convert(v, p)
  }
}

mod cb;
mod sb;

use common::{proto, version::ProtocolVersion};

use crate::packet::Packet;

pub struct Generator {
  cb: cb::Generator,
  // sb: sb::Generator,
}

impl Generator {
  pub fn new() -> Generator {
    Generator { cb: cb::Generator::new() }
  }
  pub fn clientbound(&self, v: ProtocolVersion, p: proto::Packet) -> Packet {
    self.cb.convert(v, p)
  }
}

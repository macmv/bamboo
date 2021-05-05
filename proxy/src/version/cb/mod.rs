use common::{proto, version::ProtocolVersion};
use std::collections::HashMap;

use crate::packet::Packet;

mod v1_8;

struct PacketSpec {
  gens: Vec<Box<dyn Fn(proto::Packet) -> Packet>>,
}

pub(super) struct Generator {
  gens: HashMap<ProtocolVersion, PacketSpec>,
}

impl Generator {
  pub fn new() -> Generator {
    let mut gens = HashMap::new();
    gens.insert(ProtocolVersion::V1_8, v1_8::gen_spec());
    Generator { gens }
  }

  pub fn convert(&self, v: ProtocolVersion, p: proto::Packet) -> Packet {
    self.gens[&v].gens[p.id as usize](p)
  }
}

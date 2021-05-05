use common::{net::cb, version::ProtocolVersion};
use std::{collections::HashMap, sync::Mutex};

use crate::packet::Packet;

mod v1_8;

struct PacketSpec {
  // This is keyed with protobuf packet ids, as this spec will be converting from protobuf to tcp.
  gens: HashMap<cb::ID, Box<Mutex<dyn Fn(cb::Packet) -> Packet + Send>>>,
}

impl PacketSpec {
  fn add(&mut self, id: cb::ID, f: impl Fn(cb::Packet) -> Packet + Send + 'static) {
    self.gens.insert(id, Box::new(Mutex::new(f)));
  }
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

  pub fn convert(&self, v: ProtocolVersion, p: cb::Packet) -> Packet {
    // Protocol ids are compacted, and in the same order for all versions. It is the
    // tcp id that need to be referenced with a HashMap. See sb::Generator for more.
    self.gens[&v].gens[&p.id()].lock().unwrap()(p)
  }
}

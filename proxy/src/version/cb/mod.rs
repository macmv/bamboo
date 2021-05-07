use common::{net::cb, version::ProtocolVersion};
use std::{collections::HashMap, io, io::ErrorKind, sync::Mutex};

use crate::packet::Packet;

mod v1_8;

struct PacketSpec {
  // This is keyed with protobuf packet ids, as this spec will be converting from protobuf to tcp.
  gens:
    HashMap<cb::ID, Box<Mutex<dyn Fn(cb::Packet, ProtocolVersion) -> io::Result<Packet> + Send>>>,
}

impl PacketSpec {
  fn add(
    &mut self,
    id: cb::ID,
    f: impl Fn(cb::Packet, ProtocolVersion) -> io::Result<Packet> + Send + 'static,
  ) {
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

  pub fn convert(&self, v: ProtocolVersion, p: cb::Packet) -> io::Result<Packet> {
    match self.gens.get(&v) {
      Some(g) => match g.gens.get(&p.id()) {
        Some(g) => g.lock().unwrap()(p, v),
        None => {
          Err(io::Error::new(ErrorKind::InvalidInput, format!("unknown packet {:?}", p.id())))
        }
      },
      None => Err(io::Error::new(ErrorKind::InvalidInput, format!("unknown version {:?}", v))),
    }
  }
}

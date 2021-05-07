use common::{net::sb, version::ProtocolVersion};
use std::{collections::HashMap, io, io::ErrorKind, sync::Mutex};

use crate::packet::Packet;

mod v1_8;

struct PacketSpec {
  // Each index is a tcp packet id. Each generator creates a protobuf from a tcp packet.
  gens: Vec<Box<Mutex<dyn Fn(Packet, ProtocolVersion) -> io::Result<sb::Packet> + Send>>>,
}

impl PacketSpec {
  fn add(
    &mut self,
    id: usize,
    f: impl Fn(Packet, ProtocolVersion) -> io::Result<sb::Packet> + Send + 'static,
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

  pub fn convert(&self, v: ProtocolVersion, p: Packet) -> io::Result<sb::Packet> {
    match self.gens.get(&v) {
      Some(g) => match g.gens.get(p.id() as usize) {
        Some(g) => g.lock().unwrap()(p, v),
        None => Err(io::Error::new(
          ErrorKind::InvalidInput,
          format!("got unknown packet from client {:?}", p.id()),
        )),
      },
      None => Err(io::Error::new(ErrorKind::InvalidInput, format!("unknown version {:?}", v))),
    }
  }
}

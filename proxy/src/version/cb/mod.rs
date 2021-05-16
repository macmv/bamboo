use common::{net::cb, version::ProtocolVersion};
use std::{collections::HashMap, io, io::ErrorKind, sync::Mutex};

use crate::packet::Packet;

mod v1_8;

trait PacketFn = Fn(cb::Packet, ProtocolVersion) -> io::Result<Packet> + Send;

struct PacketSpec {
  gens: HashMap<cb::ID, Box<Mutex<dyn PacketFn>>>,
}

impl PacketSpec {
  fn add(&mut self, id: cb::ID, f: impl PacketFn + 'static) {
    self.gens.insert(id, Box::new(Mutex::new(f)));
  }
}

pub(super) struct Generator {
  gens:     HashMap<ProtocolVersion, PacketSpec>,
  versions: HashMap<ProtocolVersion, Vec<data::protocol::Packet>>,
}

impl Generator {
  pub fn new(versions: HashMap<ProtocolVersion, Vec<data::protocol::Packet>>) -> Generator {
    let mut gens = HashMap::new();
    gens.insert(ProtocolVersion::V1_8, v1_8::gen_spec());
    Generator { gens, versions }
  }

  pub fn convert(&self, v: ProtocolVersion, p: cb::Packet) -> io::Result<Packet> {
    dbg!("sending packet to client: {:?}", &p);
    match self.gens.get(&v) {
      Some(g) => match g.gens.get(&p.id()) {
        Some(g) => g.lock().unwrap()(p, v),
        None => Err(io::Error::new(
          ErrorKind::InvalidInput,
          format!("got unknown packet from server {:?}", p.id()),
        )),
      },
      None => Err(io::Error::new(ErrorKind::InvalidInput, format!("unknown version {:?}", v))),
    }
  }
}

use common::{net::sb, version::ProtocolVersion};
use std::{collections::HashMap, io, io::ErrorKind, sync::Mutex};

use crate::packet::Packet;

mod v1_8;

struct PacketSpec {
  // Each index is a tcp packet id. Each generator creates a protobuf from a tcp packet.
  gens: Vec<Option<Box<Mutex<dyn Fn(Packet) -> io::Result<sb::Packet> + Send>>>>,
}

impl PacketSpec {
  fn add(&mut self, id: usize, f: impl Fn(Packet) -> io::Result<sb::Packet> + Send + 'static) {
    if id >= self.gens.len() {
      self.gens.resize_with(id + 1, || None);
    }
    self.gens[id] = Some(Box::new(Mutex::new(f)));
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
        Some(Some(g)) => g.lock().unwrap()(p),
        _ => Err(io::Error::new(
          ErrorKind::InvalidInput,
          format!("got unknown packet id from client {:#04x}", p.id()),
        )),
      },
      None => Err(io::Error::new(ErrorKind::InvalidInput, format!("unknown version {:?}", v))),
    }
  }
}

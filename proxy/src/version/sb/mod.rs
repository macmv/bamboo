use common::{net::sb, version::ProtocolVersion};
use std::{collections::HashMap, io, io::ErrorKind, sync::Mutex};

use crate::packet::Packet;

mod v1_8;

trait PacketFn = Fn(&mut Packet) -> io::Result<sb::Packet> + Send;

struct PacketSpec {
  // Each index is a tcp packet id. Each generator creates a protobuf from a tcp packet.
  gens: Vec<Option<Box<Mutex<dyn PacketFn>>>>,
}

impl PacketSpec {
  fn add(&mut self, id: usize, f: impl PacketFn + 'static) {
    if id >= self.gens.len() {
      self.gens.resize_with(id + 1, || None);
    }
    self.gens[id] = Some(Box::new(Mutex::new(f)));
  }
}

pub(super) struct Generator {
  // gens: HashMap<ProtocolVersion, PacketSpec>,
  versions: HashMap<ProtocolVersion, Vec<data::protocol::Packet>>,
}

impl Generator {
  pub fn new(versions: HashMap<ProtocolVersion, Vec<data::protocol::Packet>>) -> Generator {
    // let mut gens = HashMap::new();
    // gens.insert(ProtocolVersion::V1_8, v1_8::gen_spec());
    Generator { versions }
  }

  pub fn convert(&self, v: ProtocolVersion, mut p: Packet) -> io::Result<sb::Packet> {
    let spec = &self.versions[&v][p.id() as usize];
    dbg!(spec);
    Ok(sb::Packet::new(sb::ID::KeepAlive))
    // let packet = match self.gens.get(&v) {
    //   Some(g) => match g.gens.get(p.id() as usize) {
    //     Some(Some(g)) => g.lock().unwrap()(&mut p),
    //     _ => Err(io::Error::new(
    //       ErrorKind::InvalidInput,
    //       format!("got unknown packet id from client {:#04x}", p.id()),
    //     )),
    //   },
    //   None => Err(io::Error::new(ErrorKind::InvalidInput, format!("unknown
    // version {:?}", v))), }?;
    // if p.remaining() > 0 {
    //   Err(io::Error::new(ErrorKind::Other, format!("parser didn't read {}
    // bytes", p.remaining()))) } else {
    //   Ok(packet)
    // }
  }
}

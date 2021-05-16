use common::{net::sb, version::ProtocolVersion};
use std::{collections::HashMap, io};

use crate::packet::Packet;

trait PacketFn = Fn(&mut Packet) -> io::Result<sb::Packet> + Send;

pub(super) struct Generator {
  versions: HashMap<ProtocolVersion, Vec<data::protocol::Packet>>,
}

impl Generator {
  pub fn new(versions: HashMap<ProtocolVersion, Vec<data::protocol::Packet>>) -> Generator {
    Generator { versions }
  }

  pub fn convert(&self, v: ProtocolVersion, mut p: Packet) -> io::Result<sb::Packet> {
    let spec = &self.versions[&v][p.id() as usize];
    dbg!(p.id());
    Err(io::Error::new(io::ErrorKind::InvalidData, "gaming"))
    // Ok(sb::Packet::new(sb::ID::ChatMessage))
  }
}

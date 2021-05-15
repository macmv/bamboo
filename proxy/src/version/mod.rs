mod cb;
mod sb;

use common::{
  net::{cb::Packet as CbPacket, sb::Packet as SbPacket},
  version::ProtocolVersion,
};
use std::{collections::HashMap, io};

use crate::packet::Packet;

pub struct Generator {
  versions: HashMap<ProtocolVersion, data::protocol::Version>,
  cb:       cb::Generator<'static>,
  sb:       sb::Generator<'static>,
}

impl Default for Generator {
  fn default() -> Self {
    Generator::new()
  }
}

impl Generator {
  pub fn new() -> Generator {
    let v: HashMap<String, data::protocol::Version> =
      serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/protocol/versions.json")))
        .unwrap();
    let v = v.into_iter().map(|(k, v)| (ProtocolVersion::from_str(&k), v)).collect();
    Generator { versions: v, cb: cb::Generator::new(&v), sb: sb::Generator::new(&v) }
  }
  pub fn clientbound(&self, v: ProtocolVersion, p: CbPacket) -> io::Result<Packet> {
    self.cb.convert(v, p)
  }
  pub fn serverbound(&self, v: ProtocolVersion, p: Packet) -> io::Result<SbPacket> {
    self.sb.convert(v, p)
  }
}

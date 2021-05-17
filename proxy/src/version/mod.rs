mod cb;
mod sb;

use common::{
  net::{cb::Packet as CbPacket, sb::Packet as SbPacket},
  version::ProtocolVersion,
};
use std::{collections::HashMap, io};

use crate::packet::Packet;

pub struct Generator {
  cb: cb::Generator,
  sb: sb::Generator,
}

struct PacketVersion {
  names:   HashMap<String, usize>,
  packets: Vec<data::protocol::Packet>,
  types:   HashMap<String, data::protocol::PacketField>,
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
    let v: HashMap<ProtocolVersion, data::protocol::Version> =
      v.into_iter().map(|(k, v)| (ProtocolVersion::from_str(&k), v)).collect();
    let mut to_client = HashMap::new();
    let mut to_server = HashMap::new();
    for (k, v) in v.into_iter() {
      to_client.insert(
        k,
        PacketVersion {
          names:   v.to_client.iter().enumerate().map(|(i, p)| (p.name.clone(), i)).collect(),
          types:   v.types.clone(),
          packets: v.to_client,
        },
      );
      to_server.insert(
        k,
        PacketVersion {
          names:   v.to_server.iter().enumerate().map(|(i, p)| (p.name.clone(), i)).collect(),
          types:   v.types,
          packets: v.to_server,
        },
      );
    }
    Generator { cb: cb::Generator::new(to_client), sb: sb::Generator::new(to_server) }
  }
  pub fn clientbound(&self, v: ProtocolVersion, p: CbPacket) -> io::Result<Packet> {
    self.cb.convert(v, p)
  }
  pub fn serverbound(&self, v: ProtocolVersion, p: Packet) -> io::Result<SbPacket> {
    self.sb.convert(v, p)
  }
}

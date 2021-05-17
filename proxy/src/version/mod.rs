mod cb;
mod sb;

use common::{
  net::{
    cb::{Packet as CbPacket, ID as CbID},
    sb::{Packet as SbPacket, ID as SbID},
  },
  version::ProtocolVersion,
};
use std::{collections::HashMap, io};

use crate::packet::Packet;

pub struct Generator {
  cb: cb::Generator,
  sb: sb::Generator,
}

struct PacketVersion {
  // A list of old ids. The index in this list is a new id. The new id is generated from the ID
  // enum. Since older versions don't include all the newer packets, some of these values will be
  // None, meaning they do not exist in that version. The proxy should silently ignore packets that
  // don't exist for that client.
  ids:     Vec<Option<usize>>,
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
          ids:     generate_ids(&v.to_client, |name| CbID::from_str(name).to_i32()),
          types:   v.types.clone(),
          packets: v.to_client,
        },
      );
      to_server.insert(
        k,
        PacketVersion {
          ids:     generate_ids(&v.to_server, |name| SbID::from_str(name).to_i32()),
          types:   v.types,
          packets: v.to_server,
        },
      );
    }
    Generator { cb: cb::Generator::new(to_client), sb: sb::Generator::new(to_server) }
  }
  pub fn clientbound(&self, v: ProtocolVersion, p: CbPacket) -> io::Result<Option<Packet>> {
    self.cb.convert(v, p)
  }
  pub fn serverbound(&self, v: ProtocolVersion, p: Packet) -> io::Result<SbPacket> {
    self.sb.convert(v, p)
  }
}

fn generate_ids<F>(packets: &[data::protocol::Packet], f: F) -> Vec<Option<usize>>
where
  F: Fn(&str) -> i32,
{
  let mut ids = vec![];
  for (id, p) in packets.iter().enumerate() {
    let new_id = f(&p.name) as usize;
    if new_id >= ids.len() {
      ids.resize(new_id + 1, None);
    }
    ids[new_id] = Some(id);
  }
  ids
}

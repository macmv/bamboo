mod cb;
mod sb;

use common::{
  net::{cb as common_cb, sb as common_sb, tcp},
  version::ProtocolVersion,
};
use std::{collections::HashMap, io};

pub struct Generator {
  cb: cb::Generator,
  sb: sb::Generator,
}

struct CbPacketVersion {
  // A list of new ids. The index in this list is an old id. The new id is generated from the ID
  // enum. Since older versions don't include all the newer packets, some of these values will be
  // None, meaning they do not exist in that version. The proxy should silently ignore packets that
  // don't exist for that client.
  ids:     Vec<common_cb::ID>,
  packets: Vec<data::protocol::Packet>,
  types:   HashMap<String, data::protocol::PacketField>,
}

struct SbPacketVersion {
  // A list of old ids. The index is the serverbound packet id, and the value is the old packet id.
  ids:     Vec<Option<i32>>,
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
      v.into_iter().map(|(k, v)| (ProtocolVersion::parse_str(&k), v)).collect();
    let mut to_client = HashMap::new();
    let mut to_server = HashMap::new();
    for (k, v) in v.into_iter() {
      to_client.insert(
        k,
        CbPacketVersion {
          ids:     v.to_client.iter().map(|p| common_cb::ID::parse_str(&p.name)).collect(),
          types:   v.types.clone(),
          packets: v.to_client,
        },
      );
      to_server.insert(
        k,
        SbPacketVersion {
          ids:     generate_ids(&v.to_server, |name| common_sb::ID::parse_str(name)),
          types:   v.types,
          packets: v.to_server,
        },
      );
    }

    let mut same_versions = HashMap::new();
    same_versions.insert(ProtocolVersion::V1_16, ProtocolVersion::V1_16_2);
    same_versions.insert(ProtocolVersion::V1_16_1, ProtocolVersion::V1_16_2);
    same_versions.insert(ProtocolVersion::V1_16_3, ProtocolVersion::V1_16_2);
    same_versions.insert(ProtocolVersion::V1_16_5, ProtocolVersion::V1_16_2);
    Generator {
      cb: cb::Generator::new(to_client, same_versions.clone()),
      sb: sb::Generator::new(to_server, same_versions),
    }
  }
  pub fn serverbound(&self, v: ProtocolVersion, p: common_sb::Packet) -> io::Result<tcp::Packet> {
    match self.sb.convert(v, &p) {
      Ok(v) => Ok(v),
      Err(e) => Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("error while parsing packet {}: {}", &p, e),
      )),
    }
  }
  pub fn clientbound(
    &self,
    v: ProtocolVersion,
    mut p: tcp::Packet,
  ) -> io::Result<Vec<common_cb::Packet>> {
    self.cb.convert(v, &mut p)
  }
}

fn generate_ids<F>(packets: &[data::protocol::Packet], f: F) -> Vec<Option<i32>>
where
  F: Fn(&str) -> common_sb::ID,
{
  let mut ids = vec![];
  for (id, p) in packets.iter().enumerate() {
    let new_id = f(&p.name).to_i32() as usize;
    if new_id >= ids.len() {
      ids.resize(new_id + 1, None);
    }
    ids[new_id] = Some(id as i32);
  }
  ids
}

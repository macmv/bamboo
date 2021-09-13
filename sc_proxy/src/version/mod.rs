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

/*
struct PacketVersion {
  // A list of old ids. The index in this list is a new id. The new id is generated from the ID
  // enum. Since older versions don't include all the newer packets, some of these values will be
  // None, meaning they do not exist in that version. The proxy should silently ignore packets that
  // don't exist for that client.
  ids:     Vec<Option<usize>>,
  packets: Vec<data::protocol::Packet>,
  /* We might want to implement this field. This is a table of all complex types
   * defined in the json. For now, it is unused.
   * types:   HashMap<String, data::protocol::PacketField> */
}
*/

impl Default for Generator {
  fn default() -> Self {
    Generator::new()
  }
}

impl Generator {
  pub fn new() -> Generator {
    // let v: HashMap<String, data::protocol::Version> =
    //   serde_json::from_str(include_str!(concat!(env!("OUT_DIR"),
    // "/protocol/versions.json")))     .unwrap();
    // let v: HashMap<ProtocolVersion, data::protocol::Version> =
    //   v.into_iter().map(|(k, v)| (ProtocolVersion::parse_str(&k), v)).collect();
    // let mut to_client = HashMap::new();
    // let mut to_server = HashMap::new();
    // for (k, v) in v.into_iter() {
    //   to_client.insert(
    //     k,
    //     PacketVersion {
    //       ids:     generate_ids(&v.to_client, |name|
    // common_cb::ID::parse_str(name).to_i32()),       // types:
    // v.types.clone(),       packets: v.to_client,
    //     },
    //   );
    //   to_server.insert(
    //     k,
    //     PacketVersion {
    //       ids:     generate_ids(&v.to_server, |name|
    // common_sb::ID::parse_str(name).to_i32()),       // types:   v.types,
    //       packets: v.to_server,
    //     },
    //   );
    // }
    //
    // let mut same_versions = HashMap::new();
    // same_versions.insert(ProtocolVersion::V1_16, ProtocolVersion::V1_16_2);
    // same_versions.insert(ProtocolVersion::V1_16_1, ProtocolVersion::V1_16_2);
    // same_versions.insert(ProtocolVersion::V1_16_3, ProtocolVersion::V1_16_2);
    // same_versions.insert(ProtocolVersion::V1_16_5, ProtocolVersion::V1_16_2);
    Generator {
      // cb: cb::Generator::new(to_client, same_versions.clone()),
      // sb: sb::Generator::new(to_server, same_versions),
      cb: cb::Generator::new(),
      sb: sb::Generator::new(),
    }
  }
  pub fn clientbound(
    &self,
    v: ProtocolVersion,
    p: common_cb::Packet,
  ) -> io::Result<Vec<tcp::Packet>> {
    match self.cb.convert(v, &p) {
      Ok(v) => Ok(v),
      Err(e) => Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("error while parsing packet {:?}: {}", &p, e),
      )),
    }
  }
  pub fn serverbound(&self, v: ProtocolVersion, p: tcp::Packet) -> io::Result<common_sb::Packet> {
    self.sb.convert(v, p)
  }
}

// fn generate_ids<F>(packets: &[data::protocol::Packet], f: F) ->
// Vec<Option<usize>> where
//   F: Fn(&str) -> i32,
// {
//   let mut ids = vec![];
//   for (id, p) in packets.iter().enumerate() {
//     let new_id = f(&p.name) as usize;
//     if new_id >= ids.len() {
//       ids.resize(new_id + 1, None);
//     }
//     ids[new_id] = Some(id);
//   }
//   ids
// }

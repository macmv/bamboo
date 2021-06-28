use common::{
  net::{cb, tcp},
  version::ProtocolVersion,
};
use data::protocol::{CountType, FloatType, IntType, PacketField};
use std::{collections::HashMap, io};

use super::CbPacketVersion;

mod utils;
// mod v1_10;
// mod v1_12;
// mod v1_13;
// mod v1_14;
// mod v1_15;
// mod v1_16;
// mod v1_17;
mod v1_8;
// mod v1_9;

type BoxedPacketFn = Box<
  dyn Fn(&Generator, ProtocolVersion, &mut tcp::Packet) -> io::Result<Vec<cb::Packet>>
    + Send
    + Sync,
>;

struct PacketSpec {
  gens: HashMap<cb::ID, BoxedPacketFn>,
}

impl PacketSpec {
  fn add(
    &mut self,
    id: cb::ID,
    f: impl Fn(&Generator, ProtocolVersion, &mut tcp::Packet) -> io::Result<Vec<cb::Packet>>
      + Send
      + Sync
      + 'static,
  ) {
    self.gens.insert(id, Box::new(f));
  }
}

pub(super) struct Generator {
  gens:          HashMap<ProtocolVersion, PacketSpec>,
  versions:      HashMap<ProtocolVersion, CbPacketVersion>,
  same_versions: HashMap<ProtocolVersion, ProtocolVersion>,
}

impl Generator {
  pub fn new(
    versions: HashMap<ProtocolVersion, CbPacketVersion>,
    same_versions: HashMap<ProtocolVersion, ProtocolVersion>,
  ) -> Generator {
    let mut gens = HashMap::new();
    gens.insert(ProtocolVersion::V1_8, v1_8::gen_spec());
    // gens.insert(ProtocolVersion::V1_9_4, v1_9::gen_spec());
    // gens.insert(ProtocolVersion::V1_10, v1_10::gen_spec());
    // gens.insert(ProtocolVersion::V1_12_2, v1_12::gen_spec());
    // gens.insert(ProtocolVersion::V1_13_2, v1_13::gen_spec());
    // gens.insert(ProtocolVersion::V1_14_4, v1_14::gen_spec());
    // gens.insert(ProtocolVersion::V1_15_2, v1_15::gen_spec());
    // gens.insert(ProtocolVersion::V1_16_2, v1_16::gen_spec());
    // gens.insert(ProtocolVersion::V1_17, v1_17::gen_spec());
    Generator { gens, versions, same_versions }
  }

  fn get_ver(&self, v: ProtocolVersion) -> &CbPacketVersion {
    match self.versions.get(&v) {
      Some(v) => v,
      None => {
        &self.versions[match &self.same_versions.get(&v) {
          Some(v) => v,
          None => {
            error!("undefined protocol vesion: {:?}", v);
            panic!()
          }
        }]
      }
    }
  }

  fn get_gen(&self, v: ProtocolVersion) -> &PacketSpec {
    match self.gens.get(&v) {
      Some(v) => v,
      None => &self.gens[&self.same_versions[&v]],
    }
  }

  pub fn convert_id(&self, v: ProtocolVersion, id: i32) -> cb::ID {
    self.get_ver(v).ids[id as usize]
  }

  pub fn convert(&self, v: ProtocolVersion, p: &mut tcp::Packet) -> io::Result<Vec<cb::Packet>> {
    let id = self.convert_id(v, p.id());
    // Check for a generator
    let gen = self.get_gen(v);
    let out = match gen.gens.get(&id) {
      // If we have a generator for this packet, we use that instead. Generators are
      // used for things like chunk packets, which are just simpler to deserialize
      // manually.
      Some(g) => {
        // If we have a generator, we just want to use that
        g(self, v, p)?
      }
      None => {
        if id == cb::ID::None {
          warn!("got packet that has no generator and does not exist for ver {:?}: {:?}", v, id);
          return Ok(vec![]);
        }
        // Here, we must have a valid packet id, or we would have returned already.
        let spec = &self.get_ver(v).packets[id as usize];
        let mut out = cb::Packet::new(id);
        for (n, f) in &spec.fields {
          match f {
            PacketField::Int(v) => match v {
              IntType::VarInt | IntType::OptVarInt => out.set_int(n, p.read_varint()),
              IntType::U8 | IntType::I8 => out.set_byte(n, p.read_u8()),
              IntType::U16 | IntType::I16 => out.set_short(n, p.read_i16()),
              IntType::I32 => out.set_int(n, p.read_i32()),
              IntType::I64 => out.set_long(n, p.read_u64()),
            },
            PacketField::Float(v) => match v {
              FloatType::F32 => out.set_float(n, p.read_f32()),
              FloatType::F64 => out.set_double(n, p.read_f64()),
            },
            PacketField::Bool => out.set_bool(n, p.read_bool()),
            PacketField::String => out.set_str(n, p.read_str()),
            PacketField::Position => out.set_pos(n, p.read_pos()),
            // PacketField::UUID => out.set_uuid(n, p.read_uuid()),
            PacketField::RestBuffer => out.set_byte_arr(n, p.read_all()),
            // PacketField::NBT => out.set_byte_arr(n, p.read_nbt()),
            PacketField::Array { count, value } => {
              if **value == PacketField::String {
                if *count == CountType::Typed(IntType::VarInt) {
                  let len = p.read_varint();
                  let mut out = Vec::with_capacity(len as usize);
                  for _ in 0..len {
                    out.push(p.read_str());
                  }
                } else {
                  error!(
                    "while parsing spec for {:?} packet, got invalid array count type {:?}",
                    id, count
                  )
                };
              } else {
                error!(
                  "while parsing spec for {:?} packet, got invalid array value type {:?}",
                  id, value
                )
              }
            }
            v => {
              error!("while parsing spec for {:?} packet, got invalid packet field {:?}", id, v)
            }
          }
        }
        vec![out]
      }
    };
    // println!("writing packet {:?}: {:?}", new_id, &out);
    Ok(out)
  }
}

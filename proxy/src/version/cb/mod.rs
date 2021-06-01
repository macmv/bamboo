use common::{net::cb, version::ProtocolVersion};
use data::protocol::{FloatType, IntType, PacketField};
use std::{collections::HashMap, io};

use crate::packet::Packet;

use super::PacketVersion;

mod utils;
mod v1_10;
mod v1_12;
mod v1_13;
mod v1_14;
mod v1_15;
mod v1_8;
mod v1_9;

type BoxedPacketFn =
  Box<dyn Fn(&Generator, ProtocolVersion, &cb::Packet) -> io::Result<Vec<Packet>> + Send + Sync>;

struct PacketSpec {
  gens: HashMap<cb::ID, BoxedPacketFn>,
}

impl PacketSpec {
  fn add(
    &mut self,
    id: cb::ID,
    f: impl Fn(&Generator, ProtocolVersion, &cb::Packet) -> io::Result<Vec<Packet>>
      + Send
      + Sync
      + 'static,
  ) {
    self.gens.insert(id, Box::new(f));
  }
}

pub(super) struct Generator {
  gens:     HashMap<ProtocolVersion, PacketSpec>,
  versions: HashMap<ProtocolVersion, PacketVersion>,
}

impl Generator {
  pub fn new(versions: HashMap<ProtocolVersion, PacketVersion>) -> Generator {
    let mut gens = HashMap::new();
    gens.insert(ProtocolVersion::V1_8, v1_8::gen_spec());
    gens.insert(ProtocolVersion::V1_9_4, v1_9::gen_spec());
    gens.insert(ProtocolVersion::V1_10, v1_10::gen_spec());
    gens.insert(ProtocolVersion::V1_12_2, v1_12::gen_spec());
    gens.insert(ProtocolVersion::V1_13_2, v1_13::gen_spec());
    gens.insert(ProtocolVersion::V1_14_4, v1_14::gen_spec());
    gens.insert(ProtocolVersion::V1_15_2, v1_15::gen_spec());
    Generator { gens, versions }
  }

  pub fn convert_id(&self, v: ProtocolVersion, id: cb::ID) -> i32 {
    match self.versions[&v].ids[id.to_i32() as usize] {
      Some(v) => v as i32,
      None => -1,
    }
  }

  pub fn convert(&self, v: ProtocolVersion, p: &cb::Packet) -> io::Result<Vec<Packet>> {
    let ver = &self.versions[&v];
    let new_id = p.id();
    // Check for a generator
    if self.gens.get(&v).is_none() {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("unimplemented version {:?}", v),
      ));
    }
    let out = match self.gens[&v].gens.get(&p.id()) {
      // If we have a generator for this packet, we use that instead. Generators are
      // used for things like chunk packets, which are just simpler to serialize
      // manually.
      Some(g) => {
        // If we have a generator, we just want to use that
        g(self, v, p)?
      }
      None => {
        let id = self.convert_id(v, new_id);
        if id == -1 {
          warn!("got packet that has no generator and does not exist for ver {:?}: {}", v, p);
          return Ok(vec![]);
        }
        // Here, we must have a valid packet id, or we would have returned already.
        let spec = &ver.packets[id as usize];
        let mut out = Packet::new(id, v);
        for (n, f) in &spec.fields {
          match f {
            PacketField::Int(v) => match v {
              IntType::VarInt | IntType::OptVarInt => out.write_varint(p.get_int(n)?),
              IntType::U8 | IntType::I8 => out.write_u8(p.get_byte(n)?),
              IntType::U16 | IntType::I16 => out.write_i16(p.get_short(n)?),
              IntType::I32 => out.write_i32(p.get_int(n)?),
              IntType::I64 => out.write_u64(p.get_long(n)?),
            },
            PacketField::Float(v) => match v {
              FloatType::F32 => out.write_f32(p.get_float(n)?),
              FloatType::F64 => out.write_f64(p.get_double(n)?),
            },
            PacketField::Bool => out.write_bool(p.get_bool(n)?),
            PacketField::String => out.write_str(p.get_str(n)?),
            PacketField::Position => out.write_pos(p.get_pos(n)?),
            PacketField::UUID => out.write_uuid(p.get_uuid(n)?),
            PacketField::RestBuffer => out.write_buf(p.get_byte_arr(n)?),
            v => error!("invalid packet field {:?}", v),
          }
        }
        vec![out]
      }
    };
    // println!("writing packet {:?}: {:?}", new_id, &out);
    Ok(out)
  }
}

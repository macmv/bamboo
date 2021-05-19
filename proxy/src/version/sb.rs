use common::{net::sb, version::ProtocolVersion};
use data::protocol::{FloatType, IntType, PacketField};
use std::{collections::HashMap, io};

use crate::packet::Packet;

use super::PacketVersion;

pub(super) struct Generator {
  versions: HashMap<ProtocolVersion, PacketVersion>,
}

impl Generator {
  pub fn new(versions: HashMap<ProtocolVersion, PacketVersion>) -> Generator {
    Generator { versions }
  }

  pub fn convert(&self, v: ProtocolVersion, mut p: Packet) -> io::Result<sb::Packet> {
    let ver = &self.versions[&v];
    let types = &ver.types;
    let spec = &ver.packets[p.id() as usize];
    let mut out = sb::Packet::new(sb::ID::parse_str(&spec.name));
    for (n, f) in &spec.fields {
      match f {
        PacketField::Int(v) => match v {
          IntType::VarInt | IntType::OptVarInt => out.set_int(n.into(), p.read_varint()),
          IntType::U8 | IntType::I8 => out.set_byte(n.into(), p.read_u8()),
          IntType::U16 | IntType::I16 => out.set_short(n.into(), p.read_i16()),
          IntType::I32 => out.set_int(n.into(), p.read_i32()),
          IntType::I64 => out.set_long(n.into(), p.read_u64()),
        },
        PacketField::Float(v) => match v {
          FloatType::F32 => out.set_float(n.into(), p.read_f32()),
          FloatType::F64 => out.set_double(n.into(), p.read_f64()),
        },
        PacketField::Bool => out.set_bool(n.into(), p.read_bool()),
        PacketField::RestBuffer => out.set_byte_arr(n.into(), p.read_all()),
        PacketField::String => out.set_str(n.into(), p.read_str()),
        PacketField::DefinedType(v) => match types.get(v) {
          Some(v) => {
            dbg!(n, v);
          }
          None => unreachable!("unknown defined type: {}", v),
        },
        PacketField::Position => out.set_pos(n.into(), p.read_pos()),
        v => unreachable!("invalid packet field {:?}", v),
      }
    }
    Ok(out)
  }
}

use common::{net::sb, version::ProtocolVersion};
use data::protocol::{FloatType, IntType, PacketField};
use std::{collections::HashMap, io};

use crate::packet::Packet;

use super::PacketVersion;

trait PacketFn = Fn(&mut Packet) -> io::Result<sb::Packet> + Send;

pub(super) struct Generator {
  versions: HashMap<ProtocolVersion, PacketVersion>,
}

impl Generator {
  pub fn new(versions: HashMap<ProtocolVersion, PacketVersion>) -> Generator {
    Generator { versions }
  }

  pub fn convert(&self, v: ProtocolVersion, mut p: Packet) -> io::Result<sb::Packet> {
    let spec = &self.versions[&v].packets[p.id() as usize];
    dbg!(&spec.name, p.id());
    let mut out = sb::Packet::new(sb::ID::from_str(&spec.name));
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
        v => unreachable!("invalid packet field {:?}", v),
      }
    }
    Ok(out)
  }
}

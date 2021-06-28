use common::{
  net::{sb, tcp},
  version::ProtocolVersion,
};
use data::protocol::{FloatType, IntType, PacketField};
use std::{collections::HashMap, io};

use super::SbPacketVersion;

pub(super) struct Generator {
  versions:      HashMap<ProtocolVersion, SbPacketVersion>,
  same_versions: HashMap<ProtocolVersion, ProtocolVersion>,
}

impl Generator {
  pub fn new(
    versions: HashMap<ProtocolVersion, SbPacketVersion>,
    same_versions: HashMap<ProtocolVersion, ProtocolVersion>,
  ) -> Generator {
    Generator { versions, same_versions }
  }

  fn get_ver(&self, v: ProtocolVersion) -> &SbPacketVersion {
    match self.versions.get(&v) {
      Some(v) => v,
      None => &self.versions[&self.same_versions[&v]],
    }
  }

  pub fn convert(&self, v: ProtocolVersion, p: &sb::Packet) -> io::Result<tcp::Packet> {
    let ver = &self.get_ver(v);
    let spec = &ver.packets[p.id() as usize];
    let old_id = ver.ids[p.id().to_i32() as usize].unwrap();
    let mut out = tcp::Packet::new(old_id, v);
    for (n, f) in &spec.fields {
      match f {
        PacketField::Int(v) => match v {
          IntType::VarInt | IntType::OptVarInt => out.write_varint(p.get_int(n)),
          IntType::U8 | IntType::I8 => out.write_u8(p.get_byte(n)),
          IntType::U16 | IntType::I16 => out.write_i16(p.get_short(n)),
          IntType::I32 => out.write_i32(p.get_int(n)),
          IntType::I64 => out.write_u64(p.get_long(n)),
        },
        PacketField::Float(v) => match v {
          FloatType::F32 => out.write_f32(p.get_float(n)),
          FloatType::F64 => out.write_f64(p.get_double(n)),
        },
        PacketField::Bool => out.write_bool(p.get_bool(n)),
        PacketField::RestBuffer => out.write_buf(p.get_byte_arr(n)),
        PacketField::String => out.write_str(p.get_str(n)),
        PacketField::Position => out.write_pos(p.get_pos(n)),
        PacketField::DefinedType(v) => match v {
          // "slot" => {
          //   let (id, count, nbt) = p.read_item();
          //   out.set_int(format!("{}-id", n), id);
          //   out.set_byte(format!("{}-count", n), count);
          //   out.set_byte_arr(format!("{}-nbt", n), nbt);
          // }
          v => {
            unreachable!("invalid defined type: {}", v);
          }
        },
        v => unreachable!("invalid packet field {:?}", v),
      }
    }
    Ok(out)
  }
}

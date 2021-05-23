use common::{net::cb, version::ProtocolVersion};
use data::protocol::{FloatType, IntType, PacketField};
use std::{collections::HashMap, io, sync::Mutex};

use crate::packet::Packet;

use super::PacketVersion;

mod v1_12;
mod v1_8;
mod v1_9;

type BoxedPacketFn = Box<dyn Fn(Packet, &cb::Packet) -> io::Result<Option<Packet>> + Send + Sync>;

struct PacketSpec {
  gens: HashMap<cb::ID, BoxedPacketFn>,
}

impl PacketSpec {
  fn add(
    &mut self,
    id: cb::ID,
    f: impl Fn(Packet, &cb::Packet) -> io::Result<Option<Packet>> + Send + Sync + 'static,
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
    gens.insert(ProtocolVersion::V1_12_2, v1_12::gen_spec());
    Generator { gens, versions }
  }

  pub fn convert(&self, v: ProtocolVersion, p: &cb::Packet) -> io::Result<Option<Packet>> {
    let ver = &self.versions[&v];
    let new_id = p.id();
    // This is the old id
    let id = match ver.ids[new_id.to_i32() as usize] {
      Some(v) => v,
      None => {
        warn!("got packet that does not exist for client: {}", p);
        return Ok(None);
      }
    };
    // Old id can be used to index into packets
    let spec = &ver.packets[id];
    let mut out = Packet::new(id as i32, v);
    // If we have a generator for this packet, we use that instead. Generators are
    // used for things like chunk packets, which are just simpler to serialize
    // manually.
    if let Some(g) = self.gens[&v].gens.get(&p.id()) {
      out = match g(out, p)? {
        Some(v) => v,
        None => return Ok(None),
      }
    } else {
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
          v => unreachable!("invalid packet field {:?}", v),
        }
      }
    }
    // println!("writing packet {:?}: {:?}", new_id, &out);
    Ok(Some(out))
  }
}

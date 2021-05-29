use common::{net::cb, version::ProtocolVersion};
use data::protocol::{FloatType, IntType, PacketField};
use std::{collections::HashMap, io, sync::Mutex};

use crate::packet::Packet;

use super::PacketVersion;

mod utils;
mod v1_10;
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
    gens.insert(ProtocolVersion::V1_10, v1_10::gen_spec());
    gens.insert(ProtocolVersion::V1_12_2, v1_12::gen_spec());
    Generator { gens, versions }
  }

  pub fn convert(&self, v: ProtocolVersion, p: &cb::Packet) -> io::Result<Option<Packet>> {
    let ver = &self.versions[&v];
    let new_id = p.id();
    // Check for a generator
    let g = self.gens[&v].gens.get(&p.id());
    // This is the old id
    let id: i32 = match ver.ids[new_id.to_i32() as usize] {
      Some(v) => v as i32,
      None => {
        if g.is_none() {
          warn!("got packet that has no generator and does not exist for ver {:?}: {}", v, p);
          return Ok(None);
        } else {
          -1
        }
      }
    };
    let out = match g {
      // If we have a generator for this packet, we use that instead. Generators are
      // used for things like chunk packets, which are just simpler to serialize
      // manually.
      Some(g) => {
        // If we have a generator, we may or may not have a valid packet id.
        let out = if id != -1 { Packet::new(id, v) } else { Packet::new(0, v) };
        match g(out, p)? {
          Some(v) => v,
          None => return Ok(None),
        }
      }
      None => {
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
            v => {
              // For entity metadata. I just wanted to see a mob ingame.
              out.write_u8(127);
              warn!("invalid packet field {:?}", v)
            }
          }
        }
        out
      }
    };
    // println!("writing packet {:?}: {:?}", new_id, &out);
    Ok(Some(out))
  }
}

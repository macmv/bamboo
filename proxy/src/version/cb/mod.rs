use common::{net::cb, version::ProtocolVersion};
use data::protocol::{FloatType, IntType, PacketField};
use std::{collections::HashMap, io, io::ErrorKind, sync::Mutex};

use crate::packet::Packet;

use super::PacketVersion;

mod v1_8;

trait PacketFn = Fn(cb::Packet, ProtocolVersion) -> io::Result<Packet> + Send;

struct PacketSpec {
  gens: HashMap<cb::ID, Box<Mutex<dyn PacketFn>>>,
}

impl PacketSpec {
  fn add(&mut self, id: cb::ID, f: impl PacketFn + 'static) {
    self.gens.insert(id, Box::new(Mutex::new(f)));
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
    Generator { gens, versions }
  }

  pub fn convert(&self, v: ProtocolVersion, p: &cb::Packet) -> io::Result<Option<Packet>> {
    println!("sending packet to client: {}", p);
    let ver = &self.versions[&v];
    // This is the old id
    let id = match ver.ids[p.id().to_i32() as usize] {
      Some(v) => v,
      None => {
        warn!("got packet that does not exist for client: {}", p);
        return Ok(None);
      }
    };
    // Old id can be used to index into packets
    let spec = &ver.packets[id];
    let mut out = Packet::new(0x00, v);
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
        // PacketField::DefinedType(v) => match types.get(v) {
        //   Some(v) => {
        //     dbg!(n, v);
        //   }
        //   None => unreachable!("unknown defined type: {}", v),
        // },
        v => unreachable!("invalid packet field {:?}", v),
      }
    }
    Ok(Some(out))
    // match self.gens.get(&v) {
    //   Some(g) => match g.gens.get(&p.id()) {
    //     Some(g) => g.lock().unwrap()(p, v),
    //     None => Err(io::Error::new(
    //       ErrorKind::InvalidInput,
    //       format!("got unknown packet from server {:?}", p.id()),
    //     )),
    //   },
    //   None => Err(io::Error::new(ErrorKind::InvalidInput, format!("unknown
    // version {:?}", v))), }
  }
}

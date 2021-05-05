use super::PacketSpec;
use crate::packet::Packet;

use common::proto;

pub(super) fn gen_spec() -> PacketSpec {
  let mut gens: Vec<Box<dyn Fn(proto::Packet) -> Packet>> = Vec::new();
  gens.push(Box::new(|p: proto::Packet| {
    let mut out = Packet::new(p.id);
    out.buf.write_bool(p.bools[1]);
    out
  }));
  PacketSpec { gens }
}

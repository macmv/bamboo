use std::collections::HashMap;

use super::PacketSpec;
use crate::packet::Packet;

use common::net::cb;

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::KeepAlive, |p: cb::Packet| {
    let mut out = Packet::new(0x00);
    out.buf.write_varint(p.pb().ints[0]);
    out
  });
  spec
}

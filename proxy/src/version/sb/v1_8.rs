use super::PacketSpec;
use crate::packet::Packet;

use common::{
  net::{sb, Other},
  util::Buffer,
  version::ProtocolVersion,
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: Vec::new() };
  spec.add(0x17, |mut p: Packet, v: ProtocolVersion| {
    let mut out = sb::Packet::new(sb::ID::PluginMessage);
    dbg!(p.len());
    out.set_str(0, p.read_str());
    dbg!(p.len());
    Ok(out)
  });
  spec
}

use crate::{
  chunk::paletted::Section, math::Pos, net::VersionConverter, util::Buffer,
  version::ProtocolVersion,
};
use sc_generated::net::cb::Packet;

mod v1_8;

pub fn chunk(
  x: i32,
  z: i32,
  bit_map: u16,
  sections: Vec<Section>,
  ver: ProtocolVersion,
  conv: &impl VersionConverter,
) -> Packet {
  match ver {
    ProtocolVersion::V1_8 => v1_8::chunk(x, z, bit_map, &sections, conv),
    _ => todo!("chunk on version {}", ver),
  }
}

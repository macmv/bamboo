use std::collections::HashMap;

use super::{utils, PacketSpec};

use common::net::cb;

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::MapChunk, utils::generate_1_9_chunk);
  spec
}

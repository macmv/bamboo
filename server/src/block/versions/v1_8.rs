use std::rc::Rc;

use crate::block;
use common::{registry::VersionedRegistry, version::BlockVersion};

pub fn add_blocks(r: &mut VersionedRegistry<BlockVersion, block::Kind, Rc<block::Data>>) {}

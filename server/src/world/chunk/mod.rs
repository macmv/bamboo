mod fixed;
mod paletted;
mod section;

use std::collections::HashMap;

use common::{proto, version::BlockVersion};

use section::Section;

/// A chunk column position.
pub struct Pos {
  x: i32,
  z: i32,
}

/// A chunk column. This is not clone, because that would mean duplicating an
/// entire chunk, which you probably don't want to do. If you do need to clone a
/// chunk, use [`Chunk::duplicate()`].
pub struct Chunk {
  sections: Vec<Option<Box<dyn Section + Send>>>,
}

impl Chunk {
  pub fn new(v: BlockVersion) -> Self {
    Chunk { sections: Vec::new() }
  }
  /// Generates a protobuf containing all of the chunk data. X and Z will both
  /// be 0.
  pub fn to_proto(&self) -> proto::Chunk {
    let mut sections = HashMap::new();
    for (i, s) in self.sections.iter().enumerate() {
      match s {
        Some(s) => {
          sections.insert(i as i32, s.to_proto());
        }
        None => {}
      }
    }
    proto::Chunk { sections, ..Default::default() }
  }
}

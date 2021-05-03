mod direct;
mod paletted;
mod section;

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
  sections: Vec<Box<dyn Section + Send>>,
}

impl Chunk {
  pub fn new() -> Self {
    Chunk { sections: Vec::new() }
  }
}

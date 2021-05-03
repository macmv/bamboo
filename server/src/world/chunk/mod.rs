mod direct;
mod paletted;
mod section;

use section::Section;

/// A chunk column position.
pub struct Pos {
  x: i32,
  z: i32,
}

/// A chunk column.
pub struct Chunk {
  sections: Vec<Box<dyn Section>>,
}

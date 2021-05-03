mod chunk;

use std::{collections::HashMap, sync::Mutex};

pub struct World {
  chunks: Mutex<HashMap<chunk::Pos, chunk::Chunk>>,
}

impl World {
  pub fn new() -> Self {
    World { chunks: Mutex::new(HashMap::new()) }
  }
}

mod chunk;

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

pub struct World {
  chunks: HashMap<chunk::Pos, Mutex<chunk::Chunk>>,
}

#[derive(Clone)]
pub struct WorldManager {
  worlds: Vec<Arc<World>>,
}

impl World {
  pub fn new() -> Self {
    World { chunks: HashMap::new() }
  }
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager { worlds: Vec::new() }
  }
}

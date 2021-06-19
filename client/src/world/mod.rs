use crate::graphics::MeshChunk;
use common::math::ChunkPos;
use std::{collections::HashMap, sync::RwLock};

pub struct World {
  chunks: RwLock<HashMap<ChunkPos, MeshChunk>>,
}

impl World {
  pub fn new() -> World {
    Self { chunks: RwLock::new(HashMap::new()) }
  }
}

use crate::{graphics::MeshChunk, net::Connection};
use common::math::ChunkPos;
use std::{collections::HashMap, sync::RwLock};

pub struct World {
  chunks: RwLock<HashMap<ChunkPos, MeshChunk>>,
}

impl World {
  pub fn new() -> World {
    Self { chunks: RwLock::new(HashMap::new()) }
  }

  pub async fn connect(&self, ip: &str) {
    let conn = match Connection::new(ip).await {
      Some(c) => c,
      None => return,
    };
  }
}

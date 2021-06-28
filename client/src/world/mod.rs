use crate::{graphics::MeshChunk, net::Connection, settings::Settings};
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
    let settings = Settings::new();
    let mut conn = match Connection::new(ip, &settings).await {
      Some(c) => c,
      None => return,
    };
    conn.run().await;
  }
}

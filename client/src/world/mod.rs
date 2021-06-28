use crate::{graphics::MeshChunk, net::Connection, player::MainPlayer, Settings};
use common::math::ChunkPos;
use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

pub struct World {
  chunks: RwLock<HashMap<ChunkPos, MeshChunk>>,
}

impl World {
  pub fn new() -> World {
    Self { chunks: RwLock::new(HashMap::new()) }
  }

  pub async fn connect(&self, ip: &str) {
    let settings = Settings::new();
    let conn = match Connection::new(ip, &settings).await {
      Some(c) => Arc::new(c),
      None => return,
    };
    let player = MainPlayer::new(&settings, conn.clone());
    conn.run().await.unwrap();
  }
}

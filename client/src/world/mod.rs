use crate::{
  graphics::MeshChunk,
  net::Connection,
  player::{MainPlayer, OtherPlayer},
  Settings,
};
use common::{math::ChunkPos, util::UUID};
use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

pub struct World {
  chunks:      RwLock<HashMap<ChunkPos, MeshChunk>>,
  // This will be set whenever the player is in a game.
  main_player: Option<MainPlayer>,
  // List of other players. Does not include the main player.
  players:     HashMap<UUID, OtherPlayer>,
}

impl World {
  pub fn new() -> World {
    Self {
      chunks:      RwLock::new(HashMap::new()),
      main_player: None,
      players:     HashMap::new(),
    }
  }

  pub async fn connect(&mut self, ip: &str) {
    let settings = Settings::new();
    let conn = match Connection::new(ip, &settings).await {
      Some(c) => Arc::new(c),
      None => return,
    };
    self.main_player = Some(MainPlayer::new(&settings, conn.clone()));
    tokio::spawn(async move {
      conn.run().await.unwrap();
    });
    // Render loop
    loop {}
  }
}

use crate::{
  graphics::{MeshChunk, WindowData},
  net::Connection,
  player::{MainPlayer, OtherPlayer},
  ui::UI,
  Settings,
};
use common::{math::ChunkPos, util::UUID};
use std::{
  collections::HashMap,
  sync::{Arc, Mutex, RwLock},
};

pub struct World {
  chunks:      RwLock<HashMap<ChunkPos, Mutex<MeshChunk>>>,
  // This will be set whenever the player is in a game.
  main_player: Mutex<Option<MainPlayer>>,
  // List of other players. Does not include the main player.
  players:     HashMap<UUID, OtherPlayer>,
}

impl World {
  pub fn new() -> World {
    Self {
      chunks:      RwLock::new(HashMap::new()),
      main_player: Mutex::new(None),
      players:     HashMap::new(),
    }
  }

  pub fn connect(world: Arc<Self>, ip: String, win: &WindowData, ui: &UI) {
    tokio::spawn(async move {
      let settings = Settings::new();
      let conn = match Connection::new(&ip, &settings).await {
        Some(c) => Arc::new(c),
        None => return,
      };
      world.set_main_player(Some(MainPlayer::new(&settings, conn.clone())));
      conn.run().await.unwrap();
    });
  }

  fn set_main_player(&self, player: Option<MainPlayer>) {
    *self.main_player.lock().unwrap() = player;
  }
}

use lines::Lines;
use sc_common::{math::ChunkPos, util::UUID};
use std::{
  collections::{HashMap, HashSet},
  io,
  sync::Arc,
  time::{Duration, Instant},
};
use tokio::{sync::Mutex, time};

mod lines;

pub struct Status {
  pub players:         HashMap<UUID, Player>,
  pub loaded_chunks:   HashSet<ChunkPos>,
  pub last_keep_alive: Instant,
}

pub struct Player {
  pub username: String,
  pub uuid:     UUID,
}

impl Status {
  pub fn new() -> Self {
    Status {
      players:         HashMap::new(),
      loaded_chunks:   HashSet::new(),
      last_keep_alive: Instant::now(),
    }
  }

  pub fn draw(&self) -> io::Result<()> {
    let mut lines = Lines::new();
    lines.push_left(format!("players (tab list): {}", self.players.len()));

    lines.draw()
  }

  pub fn enable_drawing(status: Arc<Mutex<Status>>) {
    tokio::spawn(async move {
      let mut int = time::interval(Duration::from_millis(50));
      loop {
        int.tick().await;

        status.lock().await.draw().unwrap();
      }
    });
  }
}

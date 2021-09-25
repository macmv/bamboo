use ansi_term::Colour;
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
  pub render_distance: i32,
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
      render_distance: 10,
    }
  }

  pub fn draw(&self) -> io::Result<()> {
    let mut lines = Lines::new();
    lines.push_left(format!("players (tab list): {}", self.players.len()));
    let duration = Instant::now().duration_since(self.last_keep_alive).as_millis();
    lines.push_left(format!(
      "last keep alive: {} ({})",
      duration,
      match duration {
        0..=1999 => Colour::Green.paint("ok"),
        2000..=29999 => Colour::Yellow.paint("delayed"),
        30000.. => Colour::Red.paint("timeout"),
      }
    ));
    lines.push_left(format!(
      "loaded chunks: {} ({})",
      self.loaded_chunks.len(),
      match self.loaded_chunks.len() {
        v if v as i32 == (self.render_distance * 2 + 1).pow(2) => Colour::Green.paint("ok"),
        _ => Colour::Red.paint(format!(
          "expected {0} x {0} ({1}), because of {2} chunk render distance",
          self.render_distance * 2 + 1,
          (self.render_distance * 2 + 1).pow(2),
          self.render_distance,
        )),
      }
    ));

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

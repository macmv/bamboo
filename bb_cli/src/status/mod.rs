use ansi_term::Colour;
use bb_common::{math::ChunkPos, util::UUID};
use lines::Lines;
use parking_lot::Mutex;
use std::{
  collections::{HashMap, HashSet},
  io,
  sync::Arc,
  thread,
  time::{Duration, Instant},
};

mod lines;

pub struct Status {
  pub players:         HashMap<UUID, Player>,
  pub loaded_chunks:   HashSet<ChunkPos>,
  pub last_keep_alive: Instant,
  pub render_distance: i32,

  pub header: String,
  pub footer: String,
  pub hotbar: String,
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
      header:          String::new(),
      footer:          String::new(),
      hotbar:          String::new(),
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

    lines.push_right(format!("header: {}", self.header));
    lines.push_right(format!("footer: {}", self.footer));
    lines.push_right(format!("hotbar: {}", self.hotbar));

    lines.draw()
  }

  pub fn enable_drawing(status: Arc<Mutex<Status>>) {
    thread::spawn(move || {
      let tick = Duration::from_millis(50);
      let mut last_tick = Instant::now();
      loop {
        thread::sleep(tick - last_tick.elapsed());
        last_tick = Instant::now();

        status.lock().draw().unwrap();
      }
    });
  }
}

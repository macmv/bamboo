use std::sync::Arc;

#[macro_use]
extern crate log;

mod graphics;
mod ui;
mod world;

use ui::UI;
use world::World;

fn main() {
  common::init("client");

  info!("initializing graphics");
  let mut win = match graphics::init() {
    Ok(v) => v,
    Err(e) => {
      error!("{}", e);
      info!("closing");
      return;
    }
  };

  let _world = World::new();
  let ui = Arc::new(UI::new(&mut win));

  info!("starting game");
  win.run(ui);
}

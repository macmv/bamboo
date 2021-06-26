use std::sync::Arc;

#[macro_use]
extern crate log;

mod graphics;
mod ui;
pub mod util;
mod world;

use graphics::Vert;
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
  let mut ui = UI::new(&mut win);

  ui.set_layout(
    ui::LayoutKind::Menu,
    ui::Layout::new()
      .button(Vert::new(-0.2, -0.14), Vert::new(0.4, 0.08))
      .button(Vert::new(-0.2, -0.04), Vert::new(0.4, 0.08))
      .button(Vert::new(-0.2, 0.06), Vert::new(0.4, 0.08)),
  );

  info!("starting game");
  win.run(Arc::new(ui));
}

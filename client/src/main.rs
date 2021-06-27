use std::{process, sync::Arc};

#[macro_use]
extern crate log;

mod graphics;
mod net;
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

  let world = World::new();
  let mut ui = UI::new(&mut win);

  ui.set_layout(
    ui::LayoutKind::Menu,
    ui::Layout::new()
      .button(Vert::new(-0.2, -0.14), Vert::new(0.4, 0.08), move || {
        world.connect("127.0.0.1:25565");
      })
      .button(Vert::new(-0.2, -0.04), Vert::new(0.4, 0.08), || info!("options"))
      .button(Vert::new(-0.2, 0.06), Vert::new(0.4, 0.08), || {
        info!("closing");
        process::exit(0)
      }),
  );

  info!("starting game");
  win.run(Arc::new(ui));
}

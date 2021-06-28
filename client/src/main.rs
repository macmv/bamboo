use std::{process, sync::Arc};

#[macro_use]
extern crate log;

mod graphics;
mod net;
pub mod player;
mod settings;
mod ui;
pub mod util;
mod world;

use graphics::Vert;
pub use settings::Settings;
pub use ui::UI;
pub use world::World;

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

  let world = Arc::new(World::new());
  let mut ui = UI::new(&mut win);

  ui.set_layout(
    ui::LayoutKind::Menu,
    ui::Layout::new()
      .button(Vert::new(-0.2, -0.14), Vert::new(0.4, 0.08), move |win, ui| {
        World::connect(world.clone(), "127.0.0.1:25565".into(), win, ui);
      })
      .button(Vert::new(-0.2, -0.04), Vert::new(0.4, 0.08), |_, _| info!("options"))
      .button(Vert::new(-0.2, 0.06), Vert::new(0.4, 0.08), |_, _| {
        info!("closing");
        process::exit(0)
      }),
  );

  info!("starting game");
  tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
    win.run(Arc::new(ui));
  });
}

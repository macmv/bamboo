#[macro_use]
extern crate log;

mod graphics;
mod world;

fn main() {
  common::init("client");

  info!("initializing graphics");
  let game_win = match graphics::init() {
    Ok(v) => v,
    Err(e) => {
      error!("{}", e);
      info!("closing");
      return;
    }
  };

  info!("starting game");
  game_win.run();
}

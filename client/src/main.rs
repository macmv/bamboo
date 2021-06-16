#[macro_use]
extern crate log;

mod graphics;

fn main() {
  common::init("client");

  if let Err(e) = graphics::init() {
    error!("{}", e);
  }
}

#[macro_use]
extern crate bb_plugin;

use bb_plugin::{
  command::Command,
  math::FPos,
  particle,
  particle::{Color, Particle},
};

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_block_place(on_place);
  let cmd = Command::new("brush");
  bb_plugin::add_command(&cmd);
}

use bb_plugin::{math::Pos, player::Player};

fn on_place(player: Player, pos: Pos) -> bool {
  player.send_particle(Particle {
    ty:            particle::Type::Dust(Color { r: 255, g: 255, b: 0 }, 1.0),
    pos:           FPos::new(pos.x as f64, pos.y as f64 + 1.0, pos.z as f64),
    offset:        FPos::new(0.0, 0.0, 0.0),
    count:         1,
    data:          0.0,
    long_distance: false,
  });
  true
}

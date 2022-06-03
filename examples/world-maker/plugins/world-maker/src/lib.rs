#[macro_use]
extern crate bb_plugin;

use bb_plugin::{
  block,
  command::Command,
  math::FPos,
  particle,
  particle::{Color, Particle},
};

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_block_place(on_place);
  bb_plugin::set_on_tick(on_tick);
  let cmd = Command::new("brush");
  bb_plugin::add_command(&cmd);
}

use bb_plugin::{math::Pos, player::Player};

fn on_place(player: Player, pos: Pos) -> bool {
  player.send_particle(Particle {
    ty:            particle::Type::BlockMarker(block::Kind::Stone.data().default_type()),
    pos:           FPos::new(pos.x as f64 + 0.5, pos.y as f64 + 1.5, pos.z as f64 + 0.5),
    offset:        FPos::new(0.0, 0.0, 0.0),
    count:         1,
    data:          0.0,
    long_distance: false,
  });
  true
}

fn on_tick() {
  for player in bb_plugin::world::World::new(0).players() {
    let pos = player.pos();
    player.send_particle(Particle {
      ty:            particle::Type::BlockMarker(block::Kind::Stone.data().default_type()),
      pos:           FPos::new(pos.x as f64 + 0.5, pos.y as f64 + 1.5, pos.z as f64 + 0.5),
      offset:        FPos::new(0.0, 0.0, 0.0),
      count:         1,
      data:          0.0,
      long_distance: false,
    });
  }
}

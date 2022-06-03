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
  let world = bb_plugin::world::World::new(0);
  for player in world.players() {
    let pos = player.pos();
    let look = player.look_as_vec();
    let from = pos + FPos::new(0.0, 1.5, 0.0);
    let to = from + FPos::new(look.x * 50.0, look.y * 50.0, look.z * 50.0);

    if let Some(pos) = world.raycast(from, to, true) {
      let given_x = 1.0;
      let given_y = 1.0;
      let z = -(look.x * given_x + look.y * given_y) / look.z;
      let vec_in_plane = FPos::new(given_x, given_y, z);
      let unit = vec_in_plane / vec_in_plane.size();
      let other_unit = unit.cross(look);

      player.send_particle(Particle {
        ty: particle::Type::Dust(Color { r: 255, g: 255, b: 255 }, 0.5),
        pos,
        offset: FPos::new(0.0, 0.0, 0.0),
        count: 1,
        data: 0.0,
        long_distance: false,
      });
      for angle in 0..30 {
        let angle = angle as f64 / 30.0 * 2.0 * std::f64::consts::PI;
        player.send_particle(Particle {
          ty:            particle::Type::Dust(Color { r: 255, g: 255, b: 255 }, 0.5),
          pos:           pos + unit * angle.cos() + other_unit * angle.sin(),
          offset:        FPos::new(0.0, 0.0, 0.0),
          count:         1,
          data:          0.0,
          long_distance: false,
        });
      }
    }
  }
}

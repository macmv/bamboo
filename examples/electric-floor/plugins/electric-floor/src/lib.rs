use bb_plugin::{
  block,
  command::{Arg, Command},
  math::{FPos, Pos},
  particle,
  player::Player,
  sync::ConstLock,
  world::World,
};
use std::sync::atomic::{AtomicBool, Ordering};

const MIN: Pos = Pos::new(-32, 114, -32);
const MAX: Pos = Pos::new(32, 114, 32);

static STARTED: AtomicBool = AtomicBool::new(false);
static CHARGE: ConstLock<[u32; ((MAX.x - MIN.x + 1) * (MAX.z - MIN.z + 1)) as usize]> =
  ConstLock::new([0; ((MAX.x - MIN.x + 1) * (MAX.z - MIN.z + 1)) as usize]);

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_tick(on_tick);
  let cmd = Command::new("start");
  bb_plugin::add_command(&cmd, cmd_start);

  let world = bb_plugin::world::World::new(0);
  for pos in MIN.to(MAX) {
    world.set_block_kind(pos, block::Kind::WhiteStainedGlass);
    world.set_block(pos.add_y(1), block::Type::air());
    world.set_block(pos.add_y(2), block::Type::air());
    world.set_block(pos.add_y(3), block::Type::air());
  }
}

fn cmd_start(_player: Option<Player>, _args: Vec<Arg>) { STARTED.store(true, Ordering::SeqCst); }

fn charge_index(pos: Pos) -> usize {
  ((pos - MIN).x * (MAX.z - MIN.z + 1) + (pos - MIN).z) as usize
}

fn on_tick() {
  if !STARTED.load(Ordering::SeqCst) {
    return;
  }
  let world = bb_plugin::world::World::new(0);
  let mut charges = CHARGE.lock();

  for pos in MIN.to(MAX) {
    let idx = charge_index(pos);
    let mut charge = charges[idx];
    if charge > 0 && charge < 100 {
      charge += 1;
      charges[idx] = charge;
    }
    let ty = world.get_block(pos).unwrap();
    let new_kind = charge_kind(charge);
    if new_kind != ty.kind() {
      world.set_block_kind(pos, new_kind);
    }
    /*
    if ty.prop("age") != 0 && ty.prop("age").int() < 7 {
      world.set_block(pos, ty.with_prop("age", ty.prop("age").int() + 1));
    }
    */
  }
  for p in world.players() {
    let pos = p.pos();
    if pos.y > (MIN.y + 1) as f64 + 0.01 {
      break;
    }
    let pos = pos.add_y(-1.0).block();
    /*
    if ty.kind() == block::Kind::Wheat && ty.prop("age") == 0 {
      world.set_block(pos, ty.with_prop("age", 1));
    }
    */
    if MIN.to(MAX).contains(pos) {
      let idx = charge_index(pos);
      let charge = charges[idx];
      if charge == 0 {
        charges[idx] = 1;
      } else if charge == 100 {
        kill(&world, &p);
      }
    }
  }
}

fn kill(world: &World, player: &Player) {
  world.spawn_particle(particle::Particle {
    pos:           player.pos(),
    offset:        FPos::new(0.5, 0.5, 0.5),
    ty:            particle::Type::Lava,
    count:         10,
    data:          0.0,
    long_distance: false,
  });
}

fn charge_kind(charge: u32) -> block::Kind {
  match charge {
    0..=19 => block::Kind::WhiteStainedGlass,
    20..=39 => block::Kind::CyanStainedGlass,
    40..=59 => block::Kind::BlueStainedGlass,
    60..=79 => block::Kind::OrangeStainedGlass,
    80..=99 => block::Kind::PurpleStainedGlass,
    100.. => block::Kind::RedStainedGlass,
  }
}

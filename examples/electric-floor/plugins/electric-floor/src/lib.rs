#[macro_use]
extern crate bb_plugin;

use bb_plugin::{
  command::{Arg, Command},
  math::Pos,
  player::Player,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const MIN: Pos = Pos::new(-1, 114, -1);
const MAX: Pos = Pos::new(1, 114, 1);

static STARTED: AtomicBool = AtomicBool::new(false);
static TICK: AtomicU32 = AtomicU32::new(0);

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_tick(on_tick);
  let cmd = Command::new("start");
  bb_plugin::add_command(&cmd, cmd_start);
}

fn cmd_start(_player: Option<Player>, _args: Vec<Arg>) { STARTED.store(true, Ordering::SeqCst); }

fn on_tick() {
  if !STARTED.load(Ordering::SeqCst) {
    return;
  }
  let t = TICK.fetch_add(1, Ordering::SeqCst);
  if t % 10 == 0 {
    let world = bb_plugin::world::World::new(0);
    for pos in MIN.to(MAX) {
      let ty = world.get_block(pos).unwrap();
      if ty.prop("stage") != 0 && ty.prop("stage").int() < 5 {
        world.set_block(pos, ty.with_prop("stage", ty.prop("stage").int() + 1));
      }
    }
  }
}

#[macro_use]
extern crate bb_plugin;

use bb_plugin::{
  block,
  command::{Arg, Command, Parser},
  math::{FPos, Pos},
  particle,
  particle::{Color, Particle},
  player::Player,
  PlayerStore,
};
use std::{
  any::Any,
  sync::atomic::{AtomicBool, Ordering},
};

const MIN: Pos = Pos::new(-32, 80, -32);
const MAX: Pos = Pos::new(32, 80, 32);

static STARTED: AtomicBool = false.into();
static TICK: AtomicU32 = 0.into();

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_tick(on_tick);
  let cmd = Command::new("start");
  bb_plugin::add_command(cmd, cmd_start);
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
      let ty = world.get_block(pos);
      if ty.prop("stage") != 0 && ty.prop("stage") < 5 {
        ty.set_prop("stage", ty.prop("stage").int() + 1);
        world.set_block(pos, ty);
      }
    }
  }
}

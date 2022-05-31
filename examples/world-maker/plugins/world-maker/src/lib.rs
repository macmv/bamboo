#[macro_use]
extern crate bb_plugin;

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_block_place(on_place);
}

use bb_plugin::{math::Pos, player::Player};

fn on_place(player: Player, pos: Pos) -> bool { true }

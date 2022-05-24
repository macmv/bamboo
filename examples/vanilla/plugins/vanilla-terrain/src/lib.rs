pub mod vanilla;

pub mod common {
  pub use bb_plugin::{chunk, math, util};
}

#[macro_use]
extern crate bb_plugin;

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::add_world_generator("vanilla", vanilla::generate_chunk);
}

use bb_plugin::util::{chat::Color, Chat};

mod vanilla;

#[macro_use]
extern crate bb_plugin;

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::add_world_generator("vanilla", vanilla::generate_chunk);
}

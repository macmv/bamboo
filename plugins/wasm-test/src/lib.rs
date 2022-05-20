use bb_plugin::{
  chunk::{paletted, Chunk},
  math::RelPos,
  util::{chat::Color, Chat},
};

#[macro_use]
extern crate bb_plugin;

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_block_place(|player, pos| {
    let mut chat = Chat::new("player: ");
    chat.add(&player.username());
    chat.add(", x: ").color(Color::Red);
    chat.add(&format!("{}, ", pos.x));
    chat.add("y: ").color(Color::Red);
    chat.add(&format!("{}, ", pos.y));
    chat.add("z: ").color(Color::Red);
    chat.add(&format!("{}", pos.z));
    let bb = bb_plugin::instance();
    bb.broadcast(chat);
    info!("hello world");
  });
  bb_plugin::add_world_generator("testing-generator", |chunk, pos| {
    chunk.set_block(RelPos::new(0, 0, 0), 4);
  });
}

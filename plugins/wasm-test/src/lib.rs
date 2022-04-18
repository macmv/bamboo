use bb_plugin::util::{chat::Color, Chat};

#[no_mangle]
extern "C" fn on_block_place(x: i32, y: i32, z: i32) {
  let mut chat = Chat::new("pos: ");
  chat.add("x: ").color(Color::Red);
  chat.add(&format!("{}, ", x));
  chat.add("y: ").color(Color::Red);
  chat.add(&format!("{}, ", y));
  chat.add("z: ").color(Color::Red);
  chat.add(&format!("{}", z));
  let bb = bb_plugin::instance();
  bb.broadcast(chat);
}

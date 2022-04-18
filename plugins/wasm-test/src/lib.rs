use bb_plugin::util::{chat::Color, Chat};

#[no_mangle]
extern "C" fn init(ret: &mut ()) {
  let msg = format!("Hello! number: 5");
  let mut chat = Chat::new(msg);
  chat.add("foo").color(Color::Red);
  let bb = bb_plugin::instance();
  bb.broadcast(chat);
}

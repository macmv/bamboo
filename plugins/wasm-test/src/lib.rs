use bb_plugin::Chat;

#[no_mangle]
extern "C" fn init(ret: &mut ()) {
  let msg = format!("Hello! number: 5");
  let bb = bb_plugin::instance();
  bb.broadcast(Chat::new(msg));
}

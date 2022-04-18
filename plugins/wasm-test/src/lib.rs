use bb_ffi::Chat;

#[no_mangle]
extern "C" fn init(ret: &mut ()) {
  let res = format!("Hello! number: 5");
  let bb = bb_ffi::instance();
  bb.broadcast(Chat::new(res));
}

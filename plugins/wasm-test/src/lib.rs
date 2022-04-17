#[no_mangle]
extern "C" fn init(v: i32, output: &mut (*const u8, u32)) {
  let res = format!("Hello! number: {v}");
  *output = (res.as_ptr(), res.len() as u32);
}

#[repr(C)]
struct Out {
  b:   bool,
  ptr: *const u8,
  len: u32,
}

#[no_mangle]
extern "C" fn init(v: i32, output: &mut Out) {
  let res = format!("Hello! number: {v}");
  *output = Out { b: false, ptr: res.as_ptr(), len: res.len() as u32 };
}

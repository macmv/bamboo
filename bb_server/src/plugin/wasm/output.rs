use super::{Output, OUT};
use wasmer::{Memory, WasmPtr};

impl Output for () {
  fn size() -> u32 { 0 }
  fn from_addr(_: &Memory, _: OUT) -> Self {}
}
impl<A: Output> Output for (A,) {
  fn size() -> u32 { A::size() }
  fn from_addr(mem: &Memory, addr: OUT) -> Self { (A::from_addr(mem, addr),) }
}
impl<A: Output, B: Output> Output for (A, B) {
  fn size() -> u32 { A::size() + B::size() }
  fn from_addr(mem: &Memory, mut addr: OUT) -> Self {
    let a = A::from_addr(mem, addr);
    addr += A::size();
    let b = B::from_addr(mem, addr);
    (a, b)
  }
}
impl<A: Output, B: Output, C: Output> Output for (A, B, C) {
  fn size() -> u32 { A::size() + B::size() + C::size() }
  fn from_addr(mem: &Memory, mut addr: OUT) -> Self {
    let a = A::from_addr(mem, addr);
    addr += A::size();
    let b = B::from_addr(mem, addr);
    addr += B::size();
    let c = C::from_addr(mem, addr);
    (a, b, c)
  }
}
impl Output for String {
  fn size() -> u32 { <(i32, i32)>::size() }
  fn from_addr(mem: &Memory, addr: OUT) -> Self {
    let (ptr, len) = <(i32, i32)>::from_addr(mem, addr);
    let ptr = WasmPtr::<u8, _>::new(ptr as u32);
    // SAFETY: The safety invariants of `get_utf8_str` say that we cannot modify the
    // memory that the &str points to, which we aren't doing. The reason I'm not
    // just using `get_utf8_string` is because the internals of that function look a
    // lot slower than the `str` variant. I have not benchmarked it, but from a
    // glance this method seems faster.
    unsafe { ptr.get_utf8_str(mem, len as u32).unwrap().into() }
  }
}
impl Output for i32 {
  fn size() -> u32 { 4 }
  fn from_addr(mem: &Memory, addr: OUT) -> Self {
    WasmPtr::<i32>::new(addr).deref(mem).unwrap().get()
  }
}
impl Output for bool {
  fn size() -> u32 { 4 }
  fn from_addr(mem: &Memory, addr: OUT) -> Self {
    WasmPtr::<i32>::new(addr).deref(mem).unwrap().get() != 0
  }
}

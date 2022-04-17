use super::{PluginImpl, ServerMessage};
use std::{error::Error, fs, path::Path};
use wasmer::{
  imports, Exports, FromToNativeWasmType, Instance, Memory, MemoryType, Module, NativeFunc, Pages,
  Store, Value, WasmPtr, WasmTypeList,
};

pub struct Plugin {
  inst: Instance,
}

/// This is the last argument to every wasm function. It is the pointer type (we
/// are using 32 bit), and it is used for the function to write it's result
/// into.
type OUT = u32;

trait Input {
  type WasmArgs: WasmTypeList;
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
    addr: OUT,
  ) -> Option<Rets>;
}
trait Output {
  fn size() -> u32;
  fn from_addr(mem: &Memory, addr: OUT) -> Self;
}

impl Output for () {
  fn size() -> u32 { 0 }
  fn from_addr(_: &Memory, _: OUT) -> Self { () }
}
impl<A: Output> Output for (A,) {
  fn size() -> u32 { A::size() }
  fn from_addr(mem: &Memory, addr: OUT) -> Self { (A::from_addr(mem, addr),) }
}
impl<A: Output, B: Output> Output for (A, B) {
  fn size() -> u32 { A::size() + B::size() }
  fn from_addr(mem: &Memory, mut addr: OUT) -> Self {
    (A::from_addr(mem, addr), {
      addr += A::size();
      B::from_addr(mem, addr)
    })
  }
}
impl Output for String {
  fn size() -> u32 { <(i32, i32)>::size() }
  fn from_addr(mem: &Memory, mut addr: OUT) -> Self {
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
  fn from_addr(mem: &Memory, addr: OUT) -> Self { mem.view::<i32>()[addr as usize / 4].get() }
}

impl Input for i32 {
  type WasmArgs = (i32, OUT);
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
    addr: OUT,
  ) -> Option<Rets> {
    native.call(*self, addr).ok()
  }
}

impl Plugin {
  pub fn new(name: String, path: &Path, output: String) -> Result<Self, Box<dyn Error>> {
    let store = Store::default();
    let module = Module::new(&store, fs::read(path.join(output))?)?;
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
    let inst = Instance::new(&module, &import_object)?;
    Ok(Plugin { inst })
  }

  fn call<I: Input, O: Output>(&self, name: &str, input: I) -> Option<O> {
    let mem = self.inst.exports.get_memory("memory").unwrap();
    // FIXME:
    // This seems stupid, but in what world is address zero valid? The plugin will
    // never allocate at address zero, and it probably won't write to it unless we
    // tell it to. Because I can't figure out a better solution, I'm just going to
    // leave it as is until its a problem.
    let out = 0_u32;
    let func = self.inst.exports.get_native_function::<I::WasmArgs, ()>(name).ok()?;
    input.call_native(&func, out).unwrap();
    Some(O::from_addr(mem, out))
  }
}

impl PluginImpl for Plugin {
  fn call(&self, ev: ServerMessage) -> Result<bool, ()> {
    let res = self.call::<i32, String>("init", 5).unwrap();
    dbg!(res);
    Ok(true)
  }
}

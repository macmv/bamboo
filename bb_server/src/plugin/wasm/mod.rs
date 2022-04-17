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

trait Input<'a> {
  type WasmArgs: WasmTypeList;
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
    addr: OUT,
  ) -> Option<Rets>;
}
trait Output<'a> {
  fn size() -> u32;
  fn from_addr(mem: &'a Memory, addr: OUT) -> Self;
}

impl<'a> Output<'a> for () {
  fn size() -> u32 { 0 }
  fn from_addr(_: &Memory, _: OUT) -> Self { () }
}
impl<'a, A: Output<'a>> Output<'a> for (A,) {
  fn size() -> u32 { A::size() }
  fn from_addr(mem: &'a Memory, addr: OUT) -> Self { (A::from_addr(mem, addr),) }
}
impl<'a, A: Output<'a>, B: Output<'a>> Output<'a> for (A, B) {
  fn size() -> u32 { A::size() + B::size() }
  fn from_addr(mem: &'a Memory, mut addr: OUT) -> Self {
    (A::from_addr(mem, addr), {
      addr += A::size();
      B::from_addr(mem, addr)
    })
  }
}
impl<'a> Output<'a> for &'a str {
  fn size() -> u32 { <(i32, i32)>::size() }
  fn from_addr(mem: &'a Memory, mut addr: OUT) -> Self {
    let (ptr, len) = <(i32, i32)>::from_addr(mem, addr);
    let ptr = WasmPtr::<u8, _>::new(ptr as u32);
    unsafe { ptr.get_utf8_str(mem, len as u32).unwrap() }
  }
}
impl<'a> Output<'a> for i32 {
  fn size() -> u32 { 4 }
  fn from_addr(mem: &Memory, addr: OUT) -> Self { mem.view::<i32>()[addr as usize / 4].get() }
}

impl<'a> Input<'a> for i32 {
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

  fn call<'a, I: Input<'a>, O: Output<'a>>(&'a self, name: &str, input: I) -> Option<O> {
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
    let res = self.call::<i32, &str>("init", 5);
    dbg!(res);
    Ok(true)
  }
}

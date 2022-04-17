use super::{Input, OUT};
use wasmer::{FromToNativeWasmType, NativeFunc, WasmTypeList};

impl<A, B> Input for (A, B)
where
  A: Input + FromToNativeWasmType + Copy,
  B: Input + FromToNativeWasmType + Copy,
{
  type WasmArgs = (A, B, OUT);
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
    addr: OUT,
  ) -> Option<Rets> {
    native.call(self.0, self.1, addr).ok()
  }
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

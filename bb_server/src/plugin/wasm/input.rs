use super::Input;
use wasmer::{FromToNativeWasmType, NativeFunc, WasmTypeList};

impl Input for () {
  type WasmArgs = ();
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Option<Rets> {
    native.call().ok()
  }
}

impl<A, B> Input for (A, B)
where
  A: Input + FromToNativeWasmType + Copy,
  B: Input + FromToNativeWasmType + Copy,
{
  type WasmArgs = (A, B);
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Option<Rets> {
    native.call(self.0, self.1).ok()
  }
}

impl<A, B, C> Input for (A, B, C)
where
  A: Input + FromToNativeWasmType + Copy,
  B: Input + FromToNativeWasmType + Copy,
  C: Input + FromToNativeWasmType + Copy,
{
  type WasmArgs = (A, B, C);
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Option<Rets> {
    native.call(self.0, self.1, self.2).ok()
  }
}

impl<A, B, C, D> Input for (A, B, C, D)
where
  A: Input + FromToNativeWasmType + Copy,
  B: Input + FromToNativeWasmType + Copy,
  C: Input + FromToNativeWasmType + Copy,
  D: Input + FromToNativeWasmType + Copy,
{
  type WasmArgs = (A, B, C, D);
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Option<Rets> {
    native.call(self.0, self.1, self.2, self.3).ok()
  }
}

impl Input for i32 {
  type WasmArgs = i32;
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Option<Rets> {
    native.call(*self).ok()
  }
}

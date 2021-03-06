use super::Input;
use bb_ffi::CUUID;
use wasmer::{FromToNativeWasmType, NativeFunc, WasmPtr, WasmTypeList};

type Result<T> = std::result::Result<T, wasmer::RuntimeError>;

impl Input for () {
  type WasmArgs = ();
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Result<Rets> {
    native.call()
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
  ) -> Result<Rets> {
    native.call(self.0, self.1)
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
  ) -> Result<Rets> {
    native.call(self.0, self.1, self.2)
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
  ) -> Result<Rets> {
    native.call(self.0, self.1, self.2, self.3)
  }
}

impl<B, C, D> Input for (CUUID, B, C, D)
where
  B: Input + FromToNativeWasmType + Copy,
  C: Input + FromToNativeWasmType + Copy,
  D: Input + FromToNativeWasmType + Copy,
{
  type WasmArgs = (u32, u32, u32, u32, B, C, D);
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Result<Rets> {
    native.call(
      self.0.bytes[0],
      self.0.bytes[1],
      self.0.bytes[2],
      self.0.bytes[3],
      self.1,
      self.2,
      self.3,
    )
  }
}

impl Input for i32 {
  type WasmArgs = i32;
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Result<Rets> {
    native.call(*self)
  }
}
impl<T: Copy> Input for WasmPtr<T> {
  type WasmArgs = WasmPtr<T>;
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Result<Rets> {
    native.call(*self)
  }
}

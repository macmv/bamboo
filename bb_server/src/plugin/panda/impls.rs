use super::IntoPanda;
use panda::runtime::{Var, VarSend};

macro_rules! unit_impls {
  [
    $( $ty:ty ),*
  ] => {
    $(
      impl IntoPanda for $ty {
        type Panda = $ty;
        fn into_panda(self) -> $ty { self }
      }
    )*
  }
}

unit_impls![bool, u8, i8, u16, i16, u32, i32, i64, f32, f64, String, Vec<Var>];

impl IntoPanda for VarSend {
  type Panda = Var;
  fn into_panda(self) -> Var { self.into_var() }
}

impl<T, U> IntoPanda for Vec<T>
where
  T: IntoPanda<Panda = U>,
  Var: From<Vec<U>>,
{
  type Panda = Vec<U>;
  fn into_panda(self) -> Vec<U> { self.into_iter().map(|v| v.into_panda()).collect() }
}

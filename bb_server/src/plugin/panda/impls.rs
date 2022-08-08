use super::IntoPanda;

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

unit_impls![bool, u8, i8, u16, i16, u32, i32, i64, f32, f64, String];

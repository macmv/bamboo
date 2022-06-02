use crate::IntoFfi;
use bb_common::math::{FPos, Pos};

impl IntoFfi for Pos {
  type Ffi = bb_ffi::CPos;

  fn into_ffi(self) -> bb_ffi::CPos { bb_ffi::CPos { x: self.x, y: self.y, z: self.z } }
}
impl IntoFfi for FPos {
  type Ffi = bb_ffi::CFPos;

  fn into_ffi(self) -> bb_ffi::CFPos { bb_ffi::CFPos { x: self.x, y: self.y, z: self.z } }
}
impl IntoFfi for bool {
  type Ffi = bb_ffi::CBool;

  fn into_ffi(self) -> bb_ffi::CBool { bb_ffi::CBool::new(self) }
}
impl<T> IntoFfi for Vec<T> {
  type Ffi = bb_ffi::CList<T>;

  fn into_ffi(self) -> bb_ffi::CList<T> { bb_ffi::CList::new(self) }
}

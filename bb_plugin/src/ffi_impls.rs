use crate::{math::Vec3, FromFfi, IntoFfi};
use bb_common::{
  math::{FPos, Pos},
  util::UUID,
};
use bb_ffi::{CBool, CFPos, CPos, CVec3, CUUID};

impl FromFfi for Pos {
  type Ffi = CPos;

  fn from_ffi(c: CPos) -> Pos { Pos { x: c.x, y: c.y, z: c.z } }
}
impl IntoFfi for Pos {
  type Ffi = CPos;

  fn into_ffi(self) -> CPos { CPos { x: self.x, y: self.y, z: self.z } }
}

impl FromFfi for FPos {
  type Ffi = CFPos;

  fn from_ffi(c: CFPos) -> FPos { FPos { x: c.x, y: c.y, z: c.z } }
}
impl IntoFfi for FPos {
  type Ffi = bb_ffi::CFPos;

  fn into_ffi(self) -> CFPos { CFPos { x: self.x, y: self.y, z: self.z } }
}

impl FromFfi for Vec3 {
  type Ffi = CVec3;

  fn from_ffi(c: CVec3) -> Vec3 { Vec3 { x: c.x, y: c.y, z: c.z } }
}
impl IntoFfi for Vec3 {
  type Ffi = bb_ffi::CVec3;

  fn into_ffi(self) -> CVec3 { CVec3 { x: self.x, y: self.y, z: self.z } }
}

impl FromFfi for bool {
  type Ffi = CBool;

  fn from_ffi(c: CBool) -> bool { c.as_bool() }
}
impl IntoFfi for bool {
  type Ffi = bb_ffi::CBool;

  fn into_ffi(self) -> bb_ffi::CBool { bb_ffi::CBool::new(self) }
}

impl<T> IntoFfi for Vec<T> {
  type Ffi = bb_ffi::CList<T>;

  fn into_ffi(self) -> bb_ffi::CList<T> { bb_ffi::CList::new(self) }
}

impl FromFfi for UUID {
  type Ffi = CUUID;

  fn from_ffi(c: CUUID) -> UUID {
    let n = (c.bytes[0] as u128)
      | (c.bytes[1] as u128) << 32
      | (c.bytes[2] as u128) << (2 * 32)
      | (c.bytes[3] as u128) << (3 * 32);
    UUID::from_u128(n)
  }
}

impl IntoFfi for UUID {
  type Ffi = CUUID;

  fn into_ffi(self) -> CUUID {
    let n = self.as_u128();
    CUUID { bytes: [n as u32, (n >> 32) as u32, (n >> (2 * 32)) as u32, (n >> (3 * 32)) as u32] }
  }
}

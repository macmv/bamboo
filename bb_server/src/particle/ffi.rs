use super::{Particle, Type};
use crate::plugin::wasm::FromFfi;
use bb_common::math::FPos;
use bb_ffi::{CParticle, CParticleType};
use bb_transfer::MessageReader;
use wasmer::Memory;

impl FromFfi for Particle {
  type Ffi = CParticle;

  fn from_ffi(mem: &Memory, ffi: CParticle) -> Self {
    Particle {
      ty:            Type::from_ffi(mem, ffi.ty),
      pos:           FPos::from_ffi(mem, ffi.pos),
      offset:        FPos::from_ffi(mem, ffi.offset),
      count:         ffi.count,
      data:          ffi.data,
      long_distance: bool::from_ffi(mem, ffi.long_distance),
    }
  }
}
impl FromFfi for Type {
  type Ffi = CParticleType;

  fn from_ffi(mem: &Memory, ffi: CParticleType) -> Self {
    let mut ty = Type::from_id(ffi.ty).unwrap();
    let data = Vec::from_ffi(mem, ffi.data);
    let mut r = MessageReader::new(&data);
    match &mut ty {
      Type::Dust(color, scale) => {
        color.r = r.read_u8().unwrap();
        color.g = r.read_u8().unwrap();
        color.b = r.read_u8().unwrap();
        *scale = r.read_f32().unwrap();
      }
      _ => {}
    }
    ty
  }
}

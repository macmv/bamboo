use super::{Particle, Type};
use crate::{
  block,
  plugin::wasm::{Env, FromFfi},
  world::WorldManager,
};
use bb_common::math::FPos;
use bb_ffi::{CParticle, CParticleType};
use bb_transfer::MessageReader;
use wasmer::Memory;

impl FromFfi for Particle {
  type Ffi = CParticle;

  fn from_ffi(env: &Env, ffi: CParticle) -> Self {
    Particle {
      ty:            Type::from_ffi(env, ffi.ty),
      pos:           FPos::from_ffi(env, ffi.pos),
      offset:        FPos::from_ffi(env, ffi.offset),
      count:         ffi.count,
      data:          ffi.data,
      long_distance: bool::from_ffi(env, ffi.long_distance),
    }
  }
}
impl FromFfi for Type {
  type Ffi = CParticleType;

  fn from_ffi(env: &Env, ffi: CParticleType) -> Self {
    let mut ty = Type::from_id(ffi.ty).unwrap();
    let data = Vec::from_ffi(env, ffi.data);
    let mut r = MessageReader::new(&data);
    match &mut ty {
      Type::Block(block) => {
        *block = env.wm.block_converter().type_from_id(r.read_u32().unwrap(), env.ver)
      }
      Type::BlockMarker(block) => {
        *block = env.wm.block_converter().type_from_id(r.read_u32().unwrap(), env.ver)
      }
      Type::FallingDust(block) => {
        *block = env.wm.block_converter().type_from_id(r.read_u32().unwrap(), env.ver)
      }
      Type::Dust(color, scale) => {
        color.r = r.read_u8().unwrap();
        color.g = r.read_u8().unwrap();
        color.b = r.read_u8().unwrap();
        *scale = r.read_f32().unwrap();
      }
      Type::DustColorTransition(from, to, scale) => {
        from.r = r.read_u8().unwrap();
        from.g = r.read_u8().unwrap();
        from.b = r.read_u8().unwrap();
        *scale = r.read_f32().unwrap();
        to.r = r.read_u8().unwrap();
        to.g = r.read_u8().unwrap();
        to.b = r.read_u8().unwrap();
      }
      _ => {}
    }
    ty
  }
}

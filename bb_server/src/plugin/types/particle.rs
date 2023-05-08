use panda::{
    parse::token::Span,
    runtime::RuntimeError,
};
use crate::{
    particle,
    particle::Particle,
};
use bb_server_macros::define_ty;
use crate::plugin::types::util::PFPos;
use std::str::FromStr;

#[define_ty]
impl PParticle {
    info! {
        wrap: Particle,

        panda: {
            path: "bamboo::particle::Particle",
        },
    }

    pub fn new(name: &str, pos: &PFPos, force: bool, offset: &PFPos, count: u32, data: f32) -> Result<Self, RuntimeError> {
        Ok(PParticle {
            inner: Particle::new(
                particle::Type::from_str(name)
                    .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?,
                pos.inner,
                force,
                offset.inner,
                count,
                data,
            ),
        })
    }
}
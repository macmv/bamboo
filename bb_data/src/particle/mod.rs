use crate::{Collector, Target};
use serde::Deserialize;
use std::{fs, io};

mod cross;
mod gen;

pub fn generate(c: &Collector, target: Target) -> io::Result<()> {
  fs::create_dir_all(c.out.join("particle"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: ParticleDef = c.dl.get("particles", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, target, &c.out.join("particle"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParticleDef {
  particles: Vec<Particle>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Particle {
  name: String,
  id:   u32,
}

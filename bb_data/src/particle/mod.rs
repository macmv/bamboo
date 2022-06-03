use crate::{dl, Target};
use serde::Deserialize;
use std::{fs, io, path::Path};

mod cross;
mod gen;

pub fn generate(out_dir: &Path, target: Target) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("particle"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: ParticleDef = dl::get("particles", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, target, &out_dir.join("particle"))?;
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

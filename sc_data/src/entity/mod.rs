use serde::Deserialize;
use std::{fs, io, path::Path};

mod gen;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("entity"))?;
  for _ver in crate::VERSIONS {
    gen::generate(EntityDef {}, &out_dir.join("entity"))?;
  }
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityDef {}

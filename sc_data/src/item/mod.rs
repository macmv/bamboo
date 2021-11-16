use crate::dl;
use serde::Deserialize;
use std::{fs, io, path::Path};

mod gen;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("item"))?;
  for &ver in crate::VERSIONS {
    gen::generate(ItemDef {}, &out_dir.join("item"))?;
  }
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDef {}

use super::BlockDef;
use crate::gen;
use std::{fs::File, io, path::Path};

pub fn generate(def: BlockDef, dir: &Path) -> io::Result<()> {
  File::create(dir.join("ty.rs"))?;
  File::create(dir.join("version.rs"))?;
  Ok(())
}

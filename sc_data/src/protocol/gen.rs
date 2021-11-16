use super::PacketDef;
use crate::gen;
use std::{fs::File, io, path::Path};

pub fn generate(def: PacketDef, dir: &Path) -> io::Result<()> {
  File::create(dir.join("cb.rs"))?;
  File::create(dir.join("sb.rs"))?;
  Ok(())
}

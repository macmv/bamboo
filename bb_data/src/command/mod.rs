use crate::{Collector, Target};
use serde::Deserialize;
use std::{fs, io};

mod cross;
mod gen;

pub fn generate(c: &Collector, target: Target) -> io::Result<()> {
  fs::create_dir_all(c.out.join("command"))?;
  let versions = crate::VERSIONS
    .iter()
    .flat_map(|&ver| {
      if ver.maj >= 19 {
        let def: CommandDef = c.dl.get("commands", ver);
        Some((ver, def))
      } else {
        None
      }
    })
    .collect();
  gen::generate(versions, target, &c.out.join("command"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandDef {
  args: Vec<Arg>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Arg {
  name:      String,
  id:        u32,
  class:     String,
  has_extra: bool,
}

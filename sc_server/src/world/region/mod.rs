//! Loads vanilla region files and Sugarcane region files from disk. The former
//! is made for vanilla compatability, and the latter is a custom protobuf
//! format which is easier to maintain.

use sc_common::util::nbt::{Tag, NBT};
use std::{fs, io, path::Path};

use super::World;

fn parse_region_name(name: &str) -> Option<(i32, i32)> {
  let mut sections = name.split('.');
  if sections.next()? != "r" {
    return None;
  }
  let x = sections.next()?.parse().ok()?;
  let z = sections.next()?.parse().ok()?;
  if sections.next()? != "mca" {
    return None;
  }
  if sections.next().is_some() {
    return None;
  }
  Some((x, z))
}

impl World {
  pub fn load_from_disk(&self, path: &Path) -> io::Result<()> {
    let chunks = path.join("region");
    for f in fs::read_dir(chunks)? {
      let f = f?;
      if f.metadata()?.is_file() {
        let path = f.path();
        let name = match path.file_name().unwrap().to_str() {
          Some(s) => s,
          None => continue,
        };
        let (x, z) = match parse_region_name(name) {
          Some(v) => v,
          None => continue,
        };
        self.load_region_file(x, z, &path)?;
      }
    }
    Ok(())
  }

  fn load_region_file(&self, x: i32, z: i32, path: &Path) -> io::Result<()> {
    let nbt = NBT::deserialize_file(fs::read(path)?);
    dbg!(&nbt);
    Ok(())
  }
}

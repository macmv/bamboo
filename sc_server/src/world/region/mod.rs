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

  fn load_region_file(&self, region_x: i32, region_z: i32, path: &Path) -> io::Result<()> {
    let data = fs::read(path)?;
    let header = &data[..8192];
    // `offset` is an offset into the file, not an offset into the chunks table.
    let chunks = &data;
    for id in 0..1024 {
      let start = id * 4;
      let num = u32::from_be_bytes(header[start..start + 4].try_into().unwrap());
      let offset: usize = ((num >> 8) & 0xffffff) as usize * 4096;
      let size: usize = (num & 0xff) as usize * 4096;
      if size == 0 {
        continue;
      }

      let chunk = &chunks[offset..offset + size];
      let header = &chunk[..5];
      let len = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
      let _compression = header[4];
      let nbt = NBT::deserialize_file(chunk[5..5 + len].to_vec());
      dbg!(&nbt);
    }
    Ok(())
  }
}

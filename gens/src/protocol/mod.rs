mod parse;

use convert_case::{Case, Casing};
use serde_derive::Deserialize;
use std::{collections::HashMap, error::Error, fs, fs::File, io::Write, path::Path};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PacketField {
  I8,
  I16,
  Varint,
}

struct Packet {
  values: HashMap<String, PacketField>,
}

struct Version {
  to_client: Vec<Packet>,
  to_server: Vec<Packet>,
}

pub fn generate(dir: &Path) -> Result<(), Box<dyn Error>> {
  let dir = Path::new(dir).join("protocol");

  // This is done at runtime of the buildscript, so this path must be relative to
  // where the buildscript is.
  let versions = parse::load_all(Path::new("../gens/minecraft-data/data/pc"))?;

  fs::create_dir_all(&dir)?;
  {
    // Generates the block kinds enum
    let mut f = File::create(&dir.join("versions.rs"))?;
    writeln!(f, "/// Auto generated block kind. This is directly generated")?;
    writeln!(f, "/// from prismarine data.")?;
  }
  Ok(())
}

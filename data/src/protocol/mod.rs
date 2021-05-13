mod json;
mod parse;

use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::{collections::HashMap, error::Error, fs, fs::File, io::Write, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PacketField {
  I8,
  I16,
  Varint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
  pub values: HashMap<String, PacketField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
  pub to_client: Vec<Packet>,
  pub to_server: Vec<Packet>,
}

pub fn store(dir: &Path) -> Result<(), Box<dyn Error>> {
  let dir = Path::new(dir).join("protocol");

  // This is done at runtime of the buildscript, so this path must be relative to
  // where the buildscript is.
  let versions = parse::load_all(Path::new("../data/minecraft-data/data/pc"))?;

  fs::create_dir_all(&dir)?;
  {
    // Generates the version json in a much more easily read format. This is much
    // faster to compile than generating source code.
    let mut f = File::create(&dir.join("versions.json"))?;
    writeln!(f, "{}", serde_json::to_string(&versions)?)?;
  }
  Ok(())
}

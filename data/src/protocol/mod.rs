mod json;
mod parse;

use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::{collections::HashMap, error::Error, fs, fs::File, io::Write, path::Path};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IntType {
  I8,
  U8,
  I16,
  I32,
  I64,
  VarInt,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FloatType {
  F32,
  F64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CountType {
  // A typed count
  Typed(IntType),
  // A hardocded count
  Fixed(u32),
  // Another protocol field should be used as the count
  Named(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BitField {
  name:   String,
  size:   u32,
  signed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PacketField {
  // Simple fields
  Native, // Should never exist
  Bool,
  Int(IntType),
  Float(FloatType),
  UUID,
  String,
  Position,

  // Sizable fields
  NBT,
  Slot,
  OptionalNBT,
  RestBuffer, // The rest of the buffer
  EntityMetadata,

  // Complicated fields
  Option(Box<PacketField>),
  Array { count: CountType, value: Box<PacketField> },
  Buffer(CountType),
  BitField(Vec<BitField>),
  Container(HashMap<String, PacketField>),
  Switch { compare_to: String, fields: HashMap<String, PacketField> },
  Mappings(HashMap<String, u32>), // Mapping of packet names to ids

  // Logical fields
  CompareTo(String),
}

impl PacketField {
  pub fn as_container(self) -> Option<HashMap<String, PacketField>> {
    match self {
      Self::Container(v) => Some(v),
      _ => None,
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
  pub name:   String,
  pub fields: HashMap<String, PacketField>,
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

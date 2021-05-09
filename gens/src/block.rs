use convert_case::{Case, Casing};
use serde_derive::Deserialize;
use std::{error::Error, fs, fs::File, io::Write, path::Path};

#[derive(Debug, Deserialize)]
struct BlockState {
  name:       String,
  #[serde(alias = "type")]
  ty:         String,
  num_values: u32,
}

#[derive(Debug, Deserialize)]
struct Block {
  id:            u32,
  #[serde(alias = "displayName")]
  display_name:  String,
  name:          String,
  hardness:      f32,
  #[serde(alias = "minStateId")]
  min_state_id:  u32,
  #[serde(alias = "maxStateId")]
  max_state_id:  u32,
  states:        Vec<BlockState>,
  // Vec of item ids
  drops:         Vec<u32>,
  diggable:      bool,
  transparent:   bool,
  #[serde(alias = "filterLight")]
  filter_light:  u32,
  #[serde(alias = "emitLight")]
  emit_light:    u32,
  #[serde(alias = "boundingBox")]
  bounding_box:  String,
  #[serde(alias = "stackSize")]
  stack_size:    u32,
  #[serde(alias = "defaultState")]
  default_state: u32,
  resistance:    f32,
}

pub fn generate(dir: &Path) -> Result<(), Box<dyn Error>> {
  let dir = Path::new(dir).join("block");

  let data: Vec<Block> =
    serde_json::from_str(include_str!("../minecraft-data/data/pc/1.16.2/blocks.json"))?;

  fs::create_dir_all(&dir)?;
  let mut f = File::create(&dir.join("kind.rs"))?;
  writeln!(f, "/// Auto generated block kind. This is directly generated")?;
  writeln!(f, "/// from prismarine data. Do not edit this.")?;
  writeln!(f, "#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]")?;
  writeln!(f, "pub enum Kind {{")?;
  for b in data {
    writeln!(f, "  {},", b.name.to_case(Case::Pascal))?;
  }
  writeln!(f, "}}")?;
  Ok(())
}

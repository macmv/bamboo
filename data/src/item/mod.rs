use convert_case::{Case, Casing};
use serde_derive::Deserialize;
use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fs,
  fs::File,
  io::Write,
  path::Path,
};

#[derive(Debug, Clone, Deserialize)]
struct Item {
  id:           u32,
  name:         String,
  #[serde(rename = "displayName")]
  display_name: String,
  #[serde(rename = "stackSize")]
  stack_size:   u32,
}

// Generates all item data. Uses the set of valid block enum names to generate
// the block to place for each item.
pub fn generate(dir: &Path, blocks: HashSet<String>) -> Result<(), Box<dyn Error>> {
  let dir = Path::new(dir).join("item");

  let versions = vec![
    load_data(include_str!("../../minecraft-data/data/pc/1.16.2/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.15.2/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.14.4/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.13.2/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.12/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.11/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.10/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.9/items.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.8/items.json"))?,
  ];
  let latest = &versions[0];

  fs::create_dir_all(&dir)?;
  {
    // Generates the block kinds enum
    let mut f = File::create(&dir.join("type.rs"))?;
    writeln!(f, "/// Auto generated item type. This is directly generated")?;
    writeln!(f, "/// from prismarine data.")?;
    writeln!(f, "#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]")?;
    writeln!(f, "pub enum Type {{")?;
    for i in latest {
      let name = i.name.to_case(Case::Pascal);
      writeln!(f, "  {},", name)?;
    }
    writeln!(f, "}}")?;
  }
  {
    // Generates the item data (things like stack size, display name)
    let mut f = File::create(&dir.join("data.rs"))?;

    // Include macro must be one statement
    writeln!(f, "{{")?;
    for i in latest {
      let mut block = i.name.to_case(Case::Pascal);
      if !blocks.contains(&block) {
        block = "Air".into();
      }
      writeln!(f, "items.push(Data{{")?;
      writeln!(f, "  display_name: \"{}\",", i.display_name)?;
      writeln!(f, "  stack_size: {},", i.stack_size)?;
      writeln!(f, "  block_to_place: block::Kind::{},", block)?;
      writeln!(f, "}});")?;
    }
    writeln!(f, "}}")?;
  }
  {
    // Generates the cross-versioning data
    //
    // This cannot be in a source file, as that would take multiple minutes (and
    // 10gb of ram) to compile. So we do a bit of pre-processing on load.
    let mut f = File::create(&dir.join("versions.csv"))?;

    let mut to_old = vec![];
    for (i, v) in versions.iter().enumerate() {
      if i == 0 {
        continue;
      }
      to_old.push(generate_conversion(latest, v));
    }
    for i in 0..latest.len() {
      writeln!(
        f,
        "{},{}",
        i,
        to_old.iter().map(|arr| arr[i].to_string()).collect::<Vec<String>>().join(",")
      )?;
    }
  }
  Ok(())
}

fn load_data(data: &str) -> Result<Vec<Item>, Box<dyn Error>> {
  let v = serde_json::from_str(data)?;
  Ok(v)
}

fn generate_conversion(latest: &[Item], old: &[Item]) -> Vec<u32> {
  let mut m = HashMap::new();
  for (i, item) in old.iter().enumerate() {
    m.insert(&item.name, i as u32);
  }
  let mut out = Vec::with_capacity(latest.len());
  for i in latest {
    out.push(*m.get(&i.name).unwrap_or(&0));
  }
  out
}

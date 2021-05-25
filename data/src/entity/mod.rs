use convert_case::{Case, Casing};
use serde_derive::Deserialize;
use std::{collections::HashMap, error::Error, fs, fs::File, io::Write, path::Path};

#[derive(Debug, Clone, Deserialize)]
struct Entity {
  id:           u32,
  #[serde(rename = "internalId")]
  internal_id:  Option<u32>,
  name:         String,
  #[serde(rename = "displayName")]
  display_name: String,
  width:        Option<f32>,
  height:       Option<f32>,
  #[serde(rename = "type")]
  ty:           String,
  #[serde(default = "String::new")]
  category:     String,
}

// Generates all entity data.
pub fn generate(dir: &Path) -> Result<(), Box<dyn Error>> {
  let dir = Path::new(dir).join("entity");

  let versions = vec![
    load_data(include_str!("../../minecraft-data/data/pc/1.16.2/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.15.2/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.14.4/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.13.2/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.12/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.11/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.10/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.9/entities.json"))?,
    load_data(include_str!("../../minecraft-data/data/pc/1.8/entities.json"))?,
  ];
  let latest = &versions[0];

  fs::create_dir_all(&dir)?;
  {
    // Generates the entity types enum
    let mut f = File::create(&dir.join("type.rs"))?;
    writeln!(f, "/// Auto generated entity type. This is directly generated")?;
    writeln!(f, "/// from prismarine data.")?;
    writeln!(f, "#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]")?;
    writeln!(f, "pub enum Type {{")?;
    writeln!(f, "  None,")?;
    for e in latest {
      let name = e.name.to_case(Case::Pascal);
      writeln!(f, "  {},", name)?;
    }
    writeln!(f, "}}")?;
  }
  {
    // Generates the entity data (things like display name, hitbox, category)
    let mut f = File::create(&dir.join("data.rs"))?;

    // Include macro must be one statement
    writeln!(f, "{{")?;
    for e in latest {
      writeln!(f, "entities.push(Data{{")?;
      writeln!(f, "  display_name: \"{}\",", e.display_name)?;
      writeln!(f, "  width: {:.1},", e.width.unwrap_or(0.0))?;
      writeln!(f, "  height: {:.1},", e.height.unwrap_or(0.0))?;
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

fn load_data(data: &str) -> Result<Vec<Entity>, Box<dyn Error>> {
  let v = serde_json::from_str(data)?;
  Ok(v)
}

fn generate_conversion(latest: &[Entity], old: &[Entity]) -> Vec<u32> {
  let mut m = HashMap::new();
  for item in old {
    // Old versions of minecraft suck. Item id 26 is just missing from 1.8.
    // WHYYYYYYYY
    m.insert(&item.name, item.id);
  }
  let mut out = Vec::with_capacity(latest.len());
  for i in latest {
    out.push(*m.get(&i.name).unwrap_or(&0));
  }
  out
}

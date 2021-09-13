use crate::util;
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
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
  let files = util::load_versions(dir, "entities.json")?;
  let dir = Path::new(dir).join("entity");

  let mut versions = vec![];
  for f in files {
    versions.push(load_data(&fs::read_to_string(f)?)?);
  }
  let latest = &versions[0];

  let mut kinds = vec![];
  for e in latest {
    kinds.push(e.name.to_case(Case::Pascal));
  }

  let mut entity_gen = vec![];
  for e in latest {
    let display_name = &e.display_name;
    let width = e.width.unwrap_or(0.0);
    let height = e.height.unwrap_or(0.0);
    entity_gen.push(quote!(
      Data{
        display_name: #display_name,
        width: #width,
        height: #height,
      }
    ));
  }

  fs::create_dir_all(&dir)?;

  let mut out = String::new();

  out.push_str("/// Auto generated entity type. This is directly generated\n");
  out.push_str("/// from prismarine data.\n");
  out.push_str("#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]\n");
  out.push_str("pub enum Type {\n");
  for kind in &kinds {
    out.push_str("  ");
    out.push_str(kind);
    out.push_str(",\n");
  }
  out.push_str("  // Must be last, so that ToPrimitive and FromPrimitive work correctly\n");
  out.push_str("  None,\n");
  out.push_str("}\n");
  out.push_str("\n");
  out.push_str("/// Generates a table from all items to any metadata that type has. This\n");
  out.push_str("/// includes things like the display name, stack size, etc.\n");
  out.push_str("pub fn generate_entities() -> &'static [Data] {\n");
  out.push_str("  &[\n");
  for gen in &entity_gen {
    out.push_str("    ");
    out.push_str(&gen.to_string());
    out.push_str(",\n");
  }
  out.push_str("  ]\n");
  out.push_str("}\n");

  fs::write(dir.join("ty.rs"), out)?;
  // {
  //   // Generates the cross-versioning data
  //   //
  //   // This cannot be in a source file, as that would take multiple minutes
  // (and   // 10gb of ram) to compile. So we do a bit of pre-processing on
  // load.   let mut f = File::create(&dir.join("versions.csv"))?;
  //
  //   let mut to_old = vec![];
  //   for (i, v) in versions.iter().enumerate() {
  //     if i == 0 {
  //       continue;
  //     }
  //     to_old.push(generate_conversion(latest, v));
  //   }
  //   for i in 0..latest.len() {
  //     writeln!(
  //       f,
  //       "{},{}",
  //       i,
  //       to_old.iter().map(|arr|
  // arr[i].to_string()).collect::<Vec<String>>().join(",")     )?;
  //   }
  // }
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

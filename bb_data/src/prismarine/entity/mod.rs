//! This uses `#[allow(dead_code)]` on some structs because it is parsing things
//! from json, and we want to error if the keys don't exist.

use crate::util;
use convert_case::{Case, Casing};

use quote::quote;
use serde_derive::Deserialize;
use std::{collections::HashMap, error::Error, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
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
    let (ver_str, _) = util::ver_str(&f);

    versions.push((load_data(&fs::read_to_string(f)?)?, ver_str));
  }
  let latest = &versions[0].0;

  let mut kinds = vec![];
  for e in latest {
    kinds.push(e.name.to_case(Case::Pascal));
  }
  let mut names = vec![];
  for b in latest {
    names.push(&b.name);
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

  let mut version_data = vec![];
  for (i, v) in versions.iter().enumerate() {
    if i == 0 {
      continue;
    }
    version_data.push(generate_version_lit(generate_conversion(latest, &v.0), &v.1));
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

  out.push_str("impl FromStr for Type {\n");
  out.push_str("  type Err = InvalidEntity;\n");
  out.push_str("  fn from_str(s: &str) -> Result<Self, Self::Err> {\n");
  out.push_str("    match s {\n");
  for (i, name) in names.iter().enumerate() {
    out.push_str("      \"");
    out.push_str(&name);
    out.push_str("\" => Ok(Self::");
    out.push_str(&kinds[i]);
    out.push_str("),\n");
  }
  out.push_str("      _ => Err(InvalidEntity(s.into())),\n");
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("}\n");

  out.push_str("impl Type {\n");
  out.push_str("  pub fn id(&self) -> u32 {\n");
  out.push_str("    match self {\n");
  for (i, ent) in latest.iter().enumerate() {
    out.push_str("      Self::");
    out.push_str(&kinds[i]);
    out.push_str(" => \n");
    out.push_str(&ent.id.to_string());
    out.push_str(",\n");
  }
  out.push_str("      Self::None => 0,");
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("}\n");

  fs::write(dir.join("ty.rs"), out)?;

  let mut out = String::new();
  out.push_str("/// Generates the cross-versioning data for items. This is how old clients\n");
  out.push_str("/// can see the same items and place the same blocks as new clients.\n");
  out.push_str("pub fn generate_versions() -> &'static [Version] {\n");
  out.push_str("  &[\n");
  for ver in version_data {
    out.push_str("    ");
    out.push_str(&ver);
    out.push_str(",\n");
  }
  out.push_str("  ]\n");
  out.push_str("}\n");

  fs::write(dir.join("version.rs"), out)?;

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
  for ent in old {
    // 1.8-1.11 use PascalCase names, while versions above that use snake_case. We
    // don't know what version we're working with, so we should always convert to
    // snake_case.
    m.insert(ent.name.to_case(Case::Snake), ent.id);
  }
  let mut out = Vec::with_capacity(latest.len());
  for i in latest {
    out.push(*m.get(&i.name).unwrap_or(&0));
  }
  out
}

fn generate_version_lit(to_old: Vec<u32>, ver: &str) -> String {
  let mut to_new: Vec<u32> = vec![];
  for (new, &old) in to_old.iter().enumerate() {
    let old: usize = old.try_into().unwrap();
    if old >= to_new.len() {
      to_new.resize(old + 1, 0);
    }
    // Sometimes, multiple new blocks map to a single old block. In these
    // situations, we want to just use the first state that was mapped. So we never
    // override anything that has a value != 0.
    if to_new[old] == 0 {
      to_new[old] = new.try_into().unwrap();
    }
  }
  let mut out = String::new();
  out.push_str("Version {\n");
  out.push_str("      to_old: &[");
  for v in to_old {
    out.push_str(&v.to_string());
    out.push_str(",");
  }
  out.push_str("],\n      to_new: &[");
  for v in to_new {
    out.push_str(&v.to_string());
    out.push_str(",");
  }
  out.push_str("],\n      ver: bb_common::version::BlockVersion::");
  out.push_str(ver);
  out.push_str("\n    }");
  out
}

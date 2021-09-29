//! This uses `#[allow(dead_code)]` on some structs because it is parsing things
//! from json, and we want to error if the keys don't exist.

use crate::util;
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use quote::quote;
use serde_derive::Deserialize;
use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fs,
  path::Path,
};

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
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
  let files = util::load_versions(dir, "items.json")?;
  let dir = Path::new(dir).join("item");

  let mut versions = vec![];
  for f in files {
    versions.push(load_data(&fs::read_to_string(f)?)?);
  }
  let latest = &versions[0];

  let mut kinds = vec![];
  for i in latest {
    kinds.push(i.name.to_case(Case::Pascal));
  }

  let mut item_gens = vec![];
  for i in latest {
    let name = i.name.to_case(Case::Pascal);
    let mut block = Ident::new(&name, Span::call_site());
    if !blocks.contains(&name) {
      block = Ident::new("Air", Span::call_site());
    }
    let display_name = i.display_name.clone();
    let stack_size = i.stack_size;
    item_gens.push(quote!(Data{
      display_name: #display_name,
      stack_size: #stack_size,
      block_to_place: block::Kind::#block,
    }));
  }

  fs::create_dir_all(&dir)?;

  let mut out = String::new();
  out.push_str("/// Auto generated item type. This is directly generated\n");
  out.push_str("/// from prismarine data.\n");
  out.push_str("#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]\n");
  out.push_str("pub enum Type {\n");
  for kind in &kinds {
    out.push_str("  ");
    out.push_str(kind);
    out.push_str(",\n");
  }
  out.push_str("}\n");
  out.push_str("\n");
  out.push_str("/// Generates a table from all items to any metadata that type has. This\n");
  out.push_str("/// includes things like the display name, stack size, etc.\n");
  out.push_str("pub fn generate_items() -> &'static [Data] {\n");
  out.push_str("  &[\n");
  for gen in &item_gens {
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

fn load_data(data: &str) -> Result<Vec<Item>, Box<dyn Error>> {
  let v = serde_json::from_str(data)?;
  Ok(v)
}

fn generate_conversion(latest: &[Item], old: &[Item]) -> Vec<u32> {
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

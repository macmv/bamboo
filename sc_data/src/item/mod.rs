use crate::dl;
use serde::Deserialize;
use std::{fs, io, path::Path};

mod cross;
mod gen;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("item"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: ItemDef = dl::get("items", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &out_dir.join("item"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDef {
  items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
  /// The id of the item.
  id:    u32,
  /// The name id, used everywhere imporant.
  name:  String,
  /// The full class of this item
  class: String,
}

use crate::Collector;
use serde::Deserialize;
use std::{fs, io};

mod cross;
mod gen;

pub fn generate(c: &Collector) -> io::Result<()> {
  fs::create_dir_all(c.out.join("item"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: ItemDef = c.dl.get("items", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &c.out.join("item"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDef {
  items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Item {
  /// The id of the item.
  id:    u32,
  /// The name id, used everywhere important.
  name:  String,
  /// The full class of this item
  class: String,
}

use crate::dl;
use serde::Deserialize;
use std::{fs, io, path::Path};

mod cross;
mod gen;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("entity"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: EntityDef = dl::get("entities", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &out_dir.join("entity"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityDef {
  entities: Vec<Option<Entity>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Entity {
  /// The id of the entity.
  id:    u32,
  /// The name of the entity.
  name:  String,
  /// The full class of this entity.
  class: String,

  category:       String,
  width:          f32,
  height:         f32,
  tracking_range: u32,
}

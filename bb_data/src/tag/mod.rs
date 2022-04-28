use crate::dl;
use serde::Deserialize;
use std::{fs, io, path::Path};

mod gen;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("tag"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: TagsDef = dl::get("tags", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &out_dir.join("tag"))?;
  Ok(())
}

#[derive(Clone, Debug, Deserialize)]
pub struct TagsDef {
  categories: Vec<TagCategory>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TagCategory {
  name:   String,
  values: Vec<Tag>,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct Tag {
  name:    String,
  replace: bool,
  values:  Vec<String>,
}

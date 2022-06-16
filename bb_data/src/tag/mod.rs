use crate::Collector;
use serde::Deserialize;
use std::{fs, io};

mod gen;

pub fn generate(c: &Collector) -> io::Result<()> {
  fs::create_dir_all(c.out.join("tag"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: TagsDef = c.dl.get("tags", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &c.out.join("tag"))?;
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

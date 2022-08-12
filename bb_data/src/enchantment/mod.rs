use crate::Collector;
use serde::Deserialize;
use std::{fs, io};

mod cross;
mod gen;

pub fn generate(c: &Collector) -> io::Result<()> {
  fs::create_dir_all(c.out.join("enchantment"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: EnchantmentDef = c.dl.get("enchantments", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &c.out.join("enchantment"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnchantmentDef {
  enchantments: Vec<Enchantment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Enchantment {
  name: String,
  id:   u32,
}

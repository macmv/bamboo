use super::{cross::cross_version, Enchantment, EnchantmentDef};
use crate::{gen::CodeGen, Version};
use convert_case::{Case, Casing};

use std::{fs, io, path::Path};

pub fn generate(def: Vec<(Version, EnchantmentDef)>, dir: &Path) -> io::Result<()> {
  fs::write(dir.join("ty.rs"), generate_ty(&def.last().unwrap().1))?;
  fs::write(dir.join("version.rs"), generate_versions(&def))?;
  Ok(())
}

pub fn generate_ty(def: &EnchantmentDef) -> String {
  let mut gen = CodeGen::new();
  gen.write_line("/// Auto generated particle kind. This is directly generated");
  gen.write_line("/// from bamboo data.");
  gen.write_line("#[derive(Debug, Clone, PartialEq)]");
  gen.write("pub enum Type ");
  gen.write_block(|gen| {
    for b in &def.enchantments {
      gen.write(&b.name.to_case(Case::Pascal));
      gen.write_line(",");
    }
  });
  gen.write_line("");
  gen.write("impl FromStr for Type ");
  gen.write_block(|gen| {
    gen.write_line("type Err = InvalidEnchantment;");
    gen.write("fn from_str(s: &str) -> Result<Self, Self::Err> ");
    gen.write_block(|gen| {
      gen.write_line("Ok(match s {");
      gen.add_indent();
      for b in &def.enchantments {
        gen.write("\"");
        gen.write(&b.name);
        gen.write("\" => Self::");
        gen.write(&b.name.to_case(Case::Pascal));
        gen.write_line(",");
      }
      gen.write_line("_ => return Err(InvalidEnchantment(s.into())),");
      gen.remove_indent();
      gen.write_line("})");
    });
  });
  gen.write("impl Type ");
  gen.write_block(|gen| {
    gen.write("pub fn to_str(&self) -> &'static str");
    gen.write_block(|gen| {
      gen.write("[");
      for b in &def.enchantments {
        gen.write("\"");
        gen.write(&b.name);
        gen.write("\",");
      }
      gen.write_line("]");
      gen.write_line("[self.id() as usize]");
    });
    gen.write("pub fn id(&self) -> u32");
    gen.write_block(|gen| {
      gen.write("match self");
      gen.write_block(|gen| {
        for (id, b) in def.enchantments.iter().enumerate() {
          gen.write("Self::");
          gen.write(&b.name.to_case(Case::Pascal));
          gen.write(" => ");
          gen.write(&id.to_string());
          gen.write_line(",");
        }
      });
    });
    gen.write("pub fn from_id(id: u32) -> Option<Self>");
    gen.write_block(|gen| {
      gen.write("match id");
      gen.write_block(|gen| {
        for (id, b) in def.enchantments.iter().enumerate() {
          gen.write(&id.to_string());
          gen.write(" => ");
          gen.write("Some(Self::");
          gen.write(&b.name.to_case(Case::Pascal));
          gen.write_line("),");
        }
        gen.write_line("_ => None,");
      });
    });
  });
  gen.write_line("/// Generates a table from all particles kinds to any item data that kind has.");
  gen.write_line("/// This does not include cross-versioning data. This includes information like");
  gen.write_line("/// the particle name, and what extra data it needs over the wire.");
  gen.write_line("///");
  gen.write_line("/// This should only be called once, and will be done internally in the");
  gen.write_line("/// `WorldManager`. This is left public as it may be moved to a separate");
  gen.write_line("/// crate in the future, as it takes a long time to generate the source");
  gen.write_line("/// files for this.");
  gen.write_line("///");
  gen.write_line("/// This function is generated at compile time. See");
  gen.write_line("/// `data/src/block/mod.rs` and `build.rs` for more.");
  gen.write("pub fn generate_kinds() -> &'static [Data]");
  gen.write_block(|gen| {
    gen.write_line("&[");
    gen.add_indent();
    for p in &def.enchantments {
      enchantment_data(gen, p);
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("]");
  });

  gen.into_output()
}

fn generate_versions(versions: &[(Version, EnchantmentDef)]) -> String {
  let mut gen = CodeGen::new();

  gen.write_line("/// Generates the cross-versioning data for particles. This is how old clients");
  gen.write_line("/// can see the same particles as newer ones.");
  gen.write_line("pub fn generate_versions() -> &'static [Version] ");
  gen.write_block(|gen| {
    gen.write_line("&[");
    gen.add_indent();
    for v in versions {
      cross_version(gen, v, versions.last().unwrap());
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("]");
  });

  gen.into_output()
}

fn enchantment_data(gen: &mut CodeGen, b: &Enchantment) {
  macro_rules! write_prop {
    ($name:ident) => {
      gen.write(concat!(stringify!($name), ": "));
      b.$name.to_lit(gen);
      gen.write_line(",");
    };
    ($name:ident: $new_name:ident) => {
      gen.write(concat!(stringify!($new_name), ": "));
      b.$name.to_lit(gen);
      gen.write_line(",");
    };
  }

  gen.write_line("Data {");
  gen.add_indent();

  write_prop!(id);
  write_prop!(name);

  gen.remove_indent();
  gen.write("}");
}

pub trait ToLit {
  fn to_lit(&self, gen: &mut CodeGen);
}

impl ToLit for u8 {
  fn to_lit(&self, gen: &mut CodeGen) { gen.write(&self.to_string()); }
}
impl ToLit for u32 {
  fn to_lit(&self, gen: &mut CodeGen) { gen.write(&self.to_string()); }
}
impl ToLit for f32 {
  fn to_lit(&self, gen: &mut CodeGen) {
    if self.fract() == 0.0 {
      gen.write(&self.to_string());
      gen.write(".0");
    } else {
      gen.write(&self.to_string());
    }
  }
}
impl ToLit for String {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write("\"");
    gen.write(self);
    gen.write("\"");
  }
}

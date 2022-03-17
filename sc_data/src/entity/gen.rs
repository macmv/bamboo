use super::{cross::cross_version, meta::entity_metadata, Entity, EntityDef};
use crate::{gen::CodeGen, Version};
use convert_case::{Case, Casing};
use std::{fs, io, path::Path};

pub fn generate(def: Vec<(Version, EntityDef)>, dir: &Path) -> io::Result<()> {
  fs::write(dir.join("ty.rs"), generate_ty(&def.last().unwrap().1))?;
  fs::write(dir.join("version.rs"), generate_versions(&def))?;
  Ok(())
}

pub fn generate_ty(def: &EntityDef) -> String {
  let mut gen = CodeGen::new();
  gen.write_line("/// Auto generated entity kind. This is directly generated");
  gen.write_line("/// from sugarcane data.");
  gen.write_line("#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ToPrimitive, FromPrimitive)]");
  gen.write("pub enum Type ");
  gen.write_block(|gen| {
    for b in &def.entities {
      let b = b.as_ref().unwrap();
      gen.write(&b.name.to_case(Case::Pascal));
      gen.write_line(",");
    }
  });
  gen.write_line("");
  gen.write("impl FromStr for Type ");
  gen.write_block(|gen| {
    gen.write_line("type Err = InvalidEntity;");
    gen.write("fn from_str(s: &str) -> Result<Self, Self::Err> ");
    gen.write_block(|gen| {
      gen.write_line("Ok(match s {");
      gen.add_indent();
      for b in &def.entities {
        let b = b.as_ref().unwrap();
        gen.write("\"");
        gen.write(&b.name);
        gen.write("\" => Self::");
        gen.write(&b.name.to_case(Case::Pascal));
        gen.write_line(",");
      }
      gen.write_line("_ => return Err(InvalidEntity(s.into())),");
      gen.remove_indent();
      gen.write_line("})");
    });
  });
  gen.write("impl Type ");
  gen.write_block(|gen| {
    gen.write("pub fn to_str(&self) -> &'static str");
    gen.write_block(|gen| {
      gen.write("[");
      for b in &def.entities {
        let b = b.as_ref().unwrap();
        gen.write("\"");
        gen.write(&b.name);
        gen.write("\",");
      }
      gen.write_line("]");
      gen.write_line("[self.id() as usize]");
    });
  });
  gen.write_line("/// Generates a table from all entity kinds to any entity data that kind has.");
  gen.write_line("/// This does not include cross-versioning data. This includes information like");
  gen.write_line("/// the entity name, hitbox, immune to fire, etc.");
  gen.write_line("///");
  gen.write_line("/// This should only be called once, and will be done internally in the");
  gen.write_line("/// [`WorldManager`](crate::world::WorldManager). This is left public as it may");
  gen.write_line("/// be moved to a seperate crate in the future, as it takes a long time to");
  gen.write_line("/// generate the source files for this.");
  gen.write_line("///");
  gen.write_line("/// This function is generated at compile time. See");
  gen.write_line("/// `data/src/entity/mod.rs` and `build.rs` for more.");
  gen.write("pub fn generate_kinds() -> &'static [Data]");
  gen.write_block(|gen| {
    gen.write_line("&[");
    gen.add_indent();
    for b in &def.entities {
      let b = b.as_ref().unwrap();
      entity_data(gen, b);
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("]");
  });

  gen.into_output()
}

fn generate_versions(versions: &[(Version, EntityDef)]) -> String {
  let mut gen = CodeGen::new();

  gen.write_line("/// Generates the cross-versioning data for entities. This is how old clients");
  gen.write_line("/// can see the same entities as newer ones.");
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

  gen.write_line("/// Generates the cross-versioning data for entity metadata. This is how old");
  gen.write_line("/// clients can see the same entities as newer ones. This is specifically used");
  gen.write_line("/// for things like what item an entity is holding, it's health, etc.");
  gen.write_line("pub fn generate_metadata() -> &'static [VersionMetadata] ");
  gen.write_block(|gen| {
    gen.write_line("&[");
    gen.add_indent();
    for (v, ent) in versions {
      entity_metadata(gen, v, ent);
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("]");
  });

  gen.into_output()
}

fn entity_data(gen: &mut CodeGen, b: &Entity) {
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

  gen.write("ty: Type::");
  gen.write(&b.name.to_case(Case::Pascal));
  gen.write_line(",");

  write_prop!(name);
  write_prop!(id);
  write_prop!(width);
  write_prop!(height);

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
      gen.write(&".0");
    } else {
      gen.write(&self.to_string());
    }
  }
}
impl ToLit for String {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write("\"");
    gen.write(&self);
    gen.write("\"");
  }
}

use super::TagsDef;
use crate::{gen::CodeGen, Version};

use std::{fs, io, path::Path};

pub fn generate(def: Vec<(Version, TagsDef)>, dir: &Path) -> io::Result<()> {
  fs::write(dir.join("tags.rs"), generate_tags(&def.last().unwrap().1))?;
  Ok(())
}

pub fn generate_tags(def: &TagsDef) -> String {
  let mut gen = CodeGen::new();
  gen.write_line("pub struct TagCategories {");
  gen.write_line("  pub block: TagCategory,");
  gen.write_line("  pub item: TagCategory,");
  gen.write_line("  pub entity_type: TagCategory,");
  gen.write_line("  pub fluid: TagCategory,");
  gen.write_line("  pub game_event: TagCategory,");
  gen.write_line("}");

  gen.write_line("pub struct TagCategory {");
  gen.write_line("  pub tags: &'static [Tag],");
  gen.write_line("}");

  gen.write_line("pub struct Tag {");
  gen.write_line("  pub name: &'static str,");
  gen.write_line("  pub values: &'static [&'static str],");
  gen.write_line("}");

  gen.write("pub fn generate_tags() -> TagCategories ");
  gen.write_block(|gen| {
    gen.write_line("TagCategories {");
    gen.add_indent();
    for cat in &def.categories {
      gen.write(&cat.name);
      gen.write_line(": TagCategory {");
      gen.add_indent();
      gen.write_line("tags: &[");
      gen.add_indent();
      for tag in &cat.values {
        gen.write_line("Tag {");
        gen.add_indent();
        gen.write("name: \"");
        gen.write(&tag.name);
        gen.write_line("\",");
        gen.write("values: &[");
        for (i, val) in tag.values.iter().enumerate() {
          gen.write("\"");
          gen.write(&val);
          gen.write("\"");
          if i != tag.values.len() - 1 {
            gen.write(", ");
          }
        }
        gen.write_line("],");
        gen.remove_indent();
        gen.write_line("},");
      }
      gen.remove_indent();
      gen.write_line("],");
      gen.remove_indent();
      gen.write_line("},");
    }
    gen.remove_indent();
    gen.write_line("}");
  });

  gen.into_output()
}

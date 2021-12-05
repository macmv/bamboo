use super::{cross::cross_version, Block, BlockDef, Material, Prop, PropKind};
use crate::{gen::CodeGen, Version};
use convert_case::{Case, Casing};

use std::{fs, io, path::Path};

pub fn generate(def: Vec<(Version, BlockDef)>, dir: &Path) -> io::Result<()> {
  fs::write(dir.join("ty.rs"), generate_ty(&def.last().unwrap().1))?;
  fs::write(dir.join("version.rs"), generate_versions(&def))?;
  Ok(())
}

pub fn generate_ty(def: &BlockDef) -> String {
  let mut gen = CodeGen::new();
  gen.write_line("/// Auto generated block kind. This is directly generated");
  gen.write_line("/// from prismarine data.");
  gen.write_line("#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ToPrimitive, FromPrimitive)]");
  gen.write("pub enum Kind ");
  gen.write_block(|gen| {
    for b in &def.blocks {
      gen.write(&b.name.to_case(Case::Pascal));
      gen.write_line(",");
    }
  });
  gen.write_line("");
  gen.write("impl FromStr for Kind ");
  gen.write_block(|gen| {
    gen.write_line("type Err = InvalidBlock;");
    gen.write("fn from_str(s: &str) -> Result<Self, Self::Err> ");
    gen.write_block(|gen| {
      gen.write_line("Ok(match s {");
      gen.add_indent();
      for b in &def.blocks {
        gen.write("\"");
        gen.write(&b.name);
        gen.write("\" => Self::");
        gen.write(&b.name.to_case(Case::Pascal));
        gen.write_line(",");
      }
      gen.write_line("_ => return Err(InvalidBlock(s.into())),");
      gen.remove_indent();
      gen.write_line("})");
    });
  });
  gen.write("impl Kind ");
  gen.write_block(|gen| {
    gen.write("pub fn to_str(&self) -> &'static str ");
    gen.write_block(|gen| {
      gen.write("[");
      for b in &def.blocks {
        gen.write("\"");
        gen.write(&b.name);
        gen.write("\",");
      }
      gen.write_line("]");
      gen.write_line("[self.id() as usize]");
    });
  });
  gen.write_line("/// Generates a table from all block kinds to any block data that kind has.");
  gen.write_line("/// This does not include cross-versioning data. This includes information like");
  gen.write_line("/// the block states, the properties it might have, and custom handlers for");
  gen.write_line("/// when the block is placed (things like making fences connect, or making");
  gen.write_line("/// stairs rotate correctly).");
  gen.write_line("///");
  gen.write_line("/// The second item returned is a lookup table for a latest block id to a block");
  gen.write_line("/// kind. It is the only way to convert a block id back into a Kind.");
  gen.write_line("///");
  gen.write_line("/// This should only be called once, and will be done internally in the");
  gen.write_line("/// [`WorldManager`](crate::world::WorldManager). This is left public as it may");
  gen.write_line("/// be moved to a seperate crate in the future, as it takes a long time to");
  gen.write_line("/// generate the source files for this.");
  gen.write_line("///");
  gen.write_line("/// This function is generated at compile time. See");
  gen.write_line("/// `data/src/block/mod.rs` and `build.rs` for more.");
  gen.write("pub fn generate_kinds() -> (&'static [Data], &'static [Kind]) ");
  gen.write_block(|gen| {
    gen.write_line("(&[");
    gen.add_indent();
    for b in &def.blocks {
      block_data(gen, b);
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("],");
    gen.write_line("&[");
    gen.add_indent();
    for b in &def.blocks {
      gen.write(&b.name);
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write_line("])");
  });

  gen.into_output()
}

fn generate_versions(versions: &[(Version, BlockDef)]) -> String {
  let mut gen = CodeGen::new();

  gen.write_line("/// Generates the cross-versioning data for blocks. This is how old clients");
  gen.write_line("/// can see the same world as new clients.");
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

fn block_data(gen: &mut CodeGen, b: &Block) {
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

  gen.write("kind: Kind::");
  gen.write(&b.name.to_case(Case::Pascal));
  gen.write_line(",");

  write_prop!(name);
  write_prop!(id: state);
  write_prop!(material);
  write_prop!(hardness);
  write_prop!(resistance);
  write_prop!(properties: props);
  write_prop!(luminance: emit_light);

  gen.write("default_props: &[");
  for (i, _) in b.properties.iter().enumerate() {
    gen.write("0");
    if i != b.properties.len() - 1 {
      gen.write(", ");
    }
  }
  gen.write_line("],");

  gen.write_line("filter_light: 0,");
  gen.write_line("drops: &[],");
  gen.write_line("bounding_box: BoundingBoxKind::Block,");
  gen.write_line("transparent: false,");

  gen.remove_indent();
  gen.write("}");
}

pub trait ToLit {
  fn to_lit(&self, gen: &mut CodeGen);
}

impl ToLit for u8 {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write(&self.to_string());
  }
}
impl ToLit for u32 {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write(&self.to_string());
  }
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
impl ToLit for Material {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write("Material::");
    gen.write(&format!("{:?}", self));
  }
}
impl<T> ToLit for Vec<T>
where
  T: ToLit,
{
  fn to_lit(&self, gen: &mut CodeGen) {
    if self.is_empty() {
      gen.write("&[]");
      return;
    }
    gen.write_line("&[");
    gen.add_indent();
    for (i, p) in self.iter().enumerate() {
      p.to_lit(gen);
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write("]");
  }
}
impl ToLit for Prop {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write_line("Prop {");
    gen.add_indent();

    gen.write("name: ");
    self.name.to_lit(gen);
    gen.write_line(",");
    gen.write("kind: ");
    self.kind.to_lit(gen);
    gen.write_line(",");

    gen.remove_indent();
    gen.write("}");
  }
}

impl ToLit for PropKind {
  fn to_lit(&self, gen: &mut CodeGen) {
    match self {
      Self::Bool => gen.write("PropKind::Bool"),
      Self::Enum(values) => {
        gen.write("PropKind::Enum(&[");
        for (i, v) in values.iter().enumerate() {
          v.to_lit(gen);
          if i != values.len() - 1 {
            gen.write(", ");
          }
        }
        gen.write("])");
      }
      Self::Int { min, max } => {
        gen.write("PropKind::Int { min: ");
        gen.write(&min.to_string());
        gen.write(", max: ");
        gen.write(&max.to_string());
        gen.write(" }");
      }
    }
  }
}

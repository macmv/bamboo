use super::{cross::cross_version, Block, BlockDef, ItemDrop, Material, Prop, PropKind};
use crate::{gen::CodeGen, Version};
use convert_case::{Case, Casing};

use std::{fs, io, path::Path};

#[cfg(test)]
use super::cross::cross_test;

#[derive(Debug, Clone, Copy)]
pub struct BlockOpts {
  pub versions: bool,
  pub data:     bool,
  pub kinds:    bool,
}

pub fn generate(def: Vec<(Version, BlockDef)>, opts: BlockOpts, dir: &Path) -> io::Result<()> {
  if opts.data || opts.kinds {
    fs::write(dir.join("ty.rs"), generate_ty(&def.last().unwrap().1, opts))?;
  }
  if opts.versions {
    fs::write(dir.join("version.rs"), generate_versions(&def))?;
  }
  Ok(())
}

pub fn generate_ty(def: &BlockDef, opts: BlockOpts) -> String {
  let mut gen = CodeGen::new();
  if opts.kinds {
    gen.write_line("/// Auto generated block kind. This is directly generated");
    gen.write_line("/// from prismarine data.");
    gen.write_line("#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]");
    gen.write("pub enum Kind ");
    gen.write_block(|gen| {
      for b in &def.blocks {
        gen.write(&b.name.to_case(Case::Pascal));
        gen.write_line(",");
      }
      gen.write_line("Custom(CustomKind),");
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
      gen.write("pub const fn to_str(&self) -> &'static str ");
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
      gen.write_line("/// Returns the kind id. This is used to index into arrays of all kinds. It");
      gen.write_line("/// is not cross version compatible. If you want the first *state* id, use");
      gen.write_line("/// [`zero_state`](Kind::zero_state) instead.");
      gen.write("pub const fn id(&self) -> u32");
      gen.write_block(|gen| {
        gen.write("match self");
        gen.write_block(|gen| {
          for (id, b) in def.blocks.iter().enumerate() {
            gen.write("Self::");
            gen.write(&b.name.to_case(Case::Pascal));
            gen.write(" => ");
            gen.write(&id.to_string());
            gen.write_line(",");
          }
          gen.write("Self::Custom(id) => id.kind_id() + ");
          gen.write(&def.blocks.len().to_string());
          gen.write_line(",");
        });
      });
      gen.write_line("/// Returns the kind for the given id. This is the id from");
      gen.write_line("/// [`id`](Kind::id), not the state returned from `zero_state`.");
      gen.write("pub const fn from_id(id: u32) -> Option<Self>");
      gen.write_block(|gen| {
        gen.write("match id");
        gen.write_block(|gen| {
          for (id, b) in def.blocks.iter().enumerate() {
            gen.write(&id.to_string());
            gen.write(" => ");
            gen.write("Some(Self::");
            gen.write(&b.name.to_case(Case::Pascal));
            gen.write_line("),");
          }
          gen.write_line("_ => None,");
        });
      });
      gen.write_line("/// Returns the first state that this block has. This is not the default");
      gen.write_line("/// state.");
      gen.write_line("///");
      gen.write_line("/// There is currently no way to convert a zero state back into a `Kind`.");
      gen.write("pub const fn zero_state(&self) -> u32");
      gen.write_block(|gen| {
        gen.write("match self");
        gen.write_block(|gen| {
          for b in def.blocks.iter() {
            gen.write("Self::");
            gen.write(&b.name.to_case(Case::Pascal));
            gen.write(" => ");
            gen.write(&b.id.to_string());
            gen.write_line(",");
          }
          gen.write_line("Self::Custom(id) => id.zero_state(),");
        });
      });
    });
  }
  if opts.data {
    gen.write_line("/// Generates a table from all block kinds to any block data that kind has.");
    gen.write_line("/// This does not include cross-versioning data. This includes information");
    gen.write_line("/// like the block states, the properties it might have, and custom");
    gen.write_line("/// handlers for when the block is placed (things like making fences connect,");
    gen.write_line("/// or making stairs rotate correctly).");
    gen.write_line("///");
    gen.write_line("/// The second item returned is a lookup table for a latest block id to a");
    gen.write_line("/// block kind. It is the only way to convert a block id back into a Kind.");
    gen.write_line("///");
    gen.write_line("/// This should only be called once, and will be done internally in the");
    gen.write_line("/// [`WorldManager`](crate::world::WorldManager). This is left public as");
    gen.write_line("/// it may be moved to a separate crate in the future, as it takes a long");
    gen.write_line("/// time to generate the source files for this.");
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
        for _ in 0..b.all_states().len() {
          gen.write("Kind::");
          gen.write(&b.name.to_case(Case::Pascal));
          gen.write_line(",");
        }
      }
      gen.remove_indent();
      gen.write_line("])");
    });
  }

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

#[cfg(test)]
pub fn test(versions: Vec<(Version, BlockDef)>) {
  for v in &versions {
    cross_test(v, versions.last().unwrap());
  }
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
  for (i, prop) in b.properties.iter().enumerate() {
    gen.write(&prop.default.to_src());
    if i != b.properties.len() - 1 {
      gen.write(", ");
    }
  }
  gen.write_line("],");

  gen.write_line("filter_light: 0,");
  gen.write("drops: ");
  b.drops.to_lit(gen);
  gen.write_line(",");
  if b.no_collision {
    gen.write_line("bounding_box: BoundingBoxKind::Empty,");
  } else {
    gen.write_line("bounding_box: BoundingBoxKind::Block,");
  }
  gen.write_line("transparent: false,");
  gen.write("tags: ");
  b.tags.to_lit(gen);
  gen.write_line(",");

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
impl ToLit for i32 {
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
    for (_i, p) in self.iter().enumerate() {
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
impl ToLit for ItemDrop {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write("ItemDrop { item: ");
    self.item.to_lit(gen);
    gen.write(", min: ");
    self.min.to_lit(gen);
    gen.write(", max: ");
    self.max.to_lit(gen);
    gen.write(" }");
  }
}

mod block;
mod fixed;
mod paletted;
mod versions;

use crate::util;
pub use block::{Block, State};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use quote::quote;
use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fs,
  path::Path,
};

#[derive(Debug)]
struct BlockVersion {
  blocks:    Vec<Block>,
  // Used to lookup block by name
  names:     HashMap<String, usize>,
  // The name of this version, used to lookup in common::version::BlockVersion.
  // This is in the format V1_15_2
  enum_name: String,
}

impl BlockVersion {
  pub fn new(name: String) -> Self {
    BlockVersion { blocks: vec![], names: HashMap::new(), enum_name: name }
  }
  pub fn add_block(&mut self, block: Block) {
    self.names.insert(block.name().to_string(), self.blocks.len());
    self.blocks.push(block);
  }
  pub fn get(&self, name: &str) -> Option<&Block> {
    Some(&self.blocks[*self.names.get(name)?])
  }
}

fn generate_versions(dir: &Path) -> Vec<BlockVersion> {
  let files = util::load_versions(dir, "blocks.json").unwrap();

  let mut versions = vec![];
  for f in files {
    let fname = f.parent().unwrap().file_name().unwrap().to_str().unwrap();
    let version_id = fname.split('.').nth(1).unwrap().parse::<i32>().unwrap();
    let ver_str = format!("V1_{}", version_id);

    if version_id < 13 {
      versions.push(fixed::load_data(ver_str, &fs::read_to_string(f).unwrap()).unwrap());
    } else {
      versions.push(paletted::load_data(ver_str, &fs::read_to_string(f).unwrap()).unwrap());
    }
  }
  versions
}

pub fn generate_kinds(dir: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
  let versions = generate_versions(dir);
  let latest = &versions[0];

  let mut kinds = HashSet::new();
  for b in &latest.blocks {
    kinds.insert(b.name().to_case(Case::Pascal));
  }
  Ok(kinds)
}

pub fn generate(dir: &Path) -> Result<(), Box<dyn Error>> {
  let versions = generate_versions(dir);
  let dir = dir.join("block");
  let latest = &versions[0];

  let mut kinds = vec![];
  for b in &latest.blocks {
    kinds.push(b.name().to_case(Case::Pascal));
  }
  let mut names = vec![];
  for b in &latest.blocks {
    names.push(b.name());
  }

  let mut block_data = vec![];
  let mut all_kinds = vec![];
  for b in &latest.blocks {
    let state = b.id();
    let default_index = b.default_index();
    let kind = Ident::new(&b.name().to_case(Case::Pascal), Span::call_site());

    let mut types = vec![];
    if b.states().is_empty() {
      types.push(quote!(Type{
        kind: Kind::#kind,
        state: #state,
      }));
      all_kinds.push(quote!(Kind::#kind).to_string());
    } else {
      for s in b.states() {
        let sid = s.id();
        all_kinds.push(quote!(Kind::#kind).to_string());
        types.push(quote!(Type{
          kind: Kind::#kind,
          state: #sid,
        }));
      }
    }

    let out = quote! {
      Data {
        state: #state,
        default_index: #default_index,
        types: &[#(#types),*],
      }
    };
    block_data.push(out);
  }

  let mut version_data = vec![];
  for (i, v) in versions.iter().enumerate() {
    if i == 0 {
      continue;
    }
    if i >= versions.len() - 5 {
      // 1.8-1.12
      version_data.push(versions::generate_old(latest, v));
    } else {
      version_data.push(versions::generate(latest, v));
    }
  }

  fs::create_dir_all(&dir)?;

  let mut out = String::new();

  out.push_str("/// Auto generated block kind. This is directly generated\n");
  out.push_str("/// from prismarine data.\n");
  out.push_str("#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ToPrimitive, FromPrimitive)]\n");
  out.push_str("pub enum Kind {\n");
  for kind in &kinds {
    out.push_str("  ");
    out.push_str(kind);
    out.push_str(",\n");
  }
  out.push_str("}\n");
  out.push_str("\n");
  out.push_str("impl FromStr for Kind {\n");
  out.push_str("  type Err = InvalidBlock;\n");
  out.push_str("  fn from_str(s: &str) -> Result<Self, Self::Err> {\n");
  out.push_str("    match s {\n");
  for (i, name) in names.iter().enumerate() {
    out.push_str("      \"");
    out.push_str(&name);
    out.push_str("\" => Ok(Self::");
    out.push_str(&kinds[i]);
    out.push_str("),\n");
  }
  out.push_str("      _ => Err(InvalidBlock(s.into())),\n");
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("}\n");
  out.push_str("impl Kind {\n");
  out.push_str("  pub fn to_str(&self) -> &'static str {\n");
  out.push_str("    [");
  for name in &names {
    out.push_str("\"");
    out.push_str(&name);
    out.push_str("\",");
  }
  out.push_str("]\n");
  out.push_str("    [self.id() as usize]\n");
  out.push_str("  }\n");
  out.push_str("}\n");
  out.push_str("/// Generates a table from all block kinds to any block data that kind has.\n");
  out.push_str("/// This does not include cross-versioning data. This includes information like\n");
  out.push_str("/// the block states, the properties it might have, and custom handlers for\n");
  out.push_str("/// when the block is placed (things like making fences connect, or making\n");
  out.push_str("/// stairs rotate correctly).\n");
  out.push_str("///\n");
  out.push_str("/// The second item returned is a lookup table for a latest block id to a block\n");
  out.push_str("/// kind. It is the only way to convert a block id back into a Kind.\n");
  out.push_str("///\n");
  out.push_str("/// This should only be called once, and will be done internally in the\n");
  out.push_str("/// [`WorldManager`](crate::world::WorldManager). This is left public as it may\n");
  out.push_str("/// be moved to a seperate crate in the future, as it takes a long time to\n");
  out.push_str("/// generate the source files for this.\n");
  out.push_str("///\n");
  out.push_str("/// This function is generated at compile time. See\n");
  out.push_str("/// `data/src/block/mod.rs` and `build.rs` for more.\n");
  out.push_str("pub fn generate_kinds() -> (&'static [Data], &'static [Kind]) {\n");
  out.push_str("  (&[\n");
  for b in block_data {
    out.push_str("    ");
    out.push_str(&b.to_string());
    out.push_str(",\n");
  }
  out.push_str("  ],\n");
  out.push_str("  &[\n");
  for b in all_kinds {
    out.push_str("    ");
    out.push_str(&b);
    out.push_str(",\n");
  }
  out.push_str("  ])\n");
  out.push_str("}\n");

  fs::write(dir.join("ty.rs"), out)?;

  let mut out = String::new();
  out.push_str("/// Generates the cross-versioning data for blocks. This is how old clients\n");
  out.push_str("/// can see the same world as new clients.\n");
  out.push_str("pub fn generate_versions() -> &'static [Version] {\n");
  out.push_str("  &[\n");
  for ver in version_data {
    out.push_str("    ");
    out.push_str(&ver.to_string());
    out.push_str(",\n");
  }
  out.push_str("  ]\n");
  out.push_str("}\n");

  fs::write(dir.join("version.rs"), out)?;

  // let out = quote! {
  //   /// Auto generated block kind. This is directly generated
  //   /// from prismarine data.
  //   #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ToPrimitive,
  // FromPrimitive)]   pub enum Kind {
  //     #(#kinds),*
  //   }
  //
  //   impl FromStr for Kind {
  //     type Err = InvalidBlock;
  //     fn from_str(s: &str) -> Result<Self, Self::Err> {
  //       match s {
  //         #(#names => Ok(Self::#kinds),)*
  //         _ => Err(InvalidBlock(s.into())),
  //       }
  //     }
  //   }
  //   pub fn names() -> &'static [&'static str; #name_len] {
  //     &[#(#names),*]
  //   }
  //
  //   /// Generates a table from all block kinds to any block data that kind has.
  // This   /// does not include cross-versioning data. This includes
  // information like the   /// block states, the properties it might have, and
  // custom handlers for when the   /// block is place (things like making
  // fences connect, or making stairs rotate   /// correctly).
  //   ///
  //   /// This should only be called once, and will be done internally in the
  //   /// [`WorldManager`](crate::world::WorldManager). This is left public as it
  // may   /// be moved to a seperate crate in the future, as it takes a long
  // time to   /// generate the source files for this.
  //   ///
  //   /// Most of this function is generated at compile time. See
  //   /// `gens/src/block/mod.rs` and `build.rs` for more.
  //   pub fn generate_kinds() -> Vec<Data> {
  //     let mut blocks = vec![];
  //     #(#block_data)*
  //     blocks
  //   }
  //
  //   /// This is the conversion table for a single old version of the game and
  // the   /// latest version. This includes a list of old ids, whose index is
  // the new   /// block id. It also contains a HashMap, which is used to
  // convert old ids into   /// new ones. This might not be fastest or most
  // memory efficient way, but it is   /// certainly the easiest. Especially
  // before 1.13, block ids are very sparse,   /// and a HashMap will be the
  // best option.   #[derive(Debug)]
  //   pub struct Version {
  //     pub(super) to_old: &'static [u32],
  //     pub(super) to_new: &'static [u32],
  //     pub(super) ver:    common::version::BlockVersion,
  //   }
  //
  //   /// Generates a table from all block kinds to any block data that kind has.
  // This   /// does not include cross-versioning data. This includes
  // information like the   /// block states, the properties it might have, and
  // custom handlers for when the   /// block is place (things like making
  // fences connect, or making stairs rotate   /// correctly).
  //   ///
  //   /// This should only be called once, and will be done internally in the
  //   /// [`WorldManager`](crate::world::WorldManager). This is left public as it
  // may   /// be moved to a seperate crate in the future, as it takes a long
  // time to   /// generate the source files for this.
  //   ///
  //   /// Most of this function is generated at compile time. See
  //   /// `gens/src/block/mod.rs` and `build.rs` for more.
  //   ///
  //   /// This Vec<Version> is in order of block versions. Use
  //   /// BlockVersion::from_index() and BlockVersion::to_index() to convert
  // between   /// indicies and block versions.
  //   pub fn generate_versions() -> &'static [Version] {
  //     &[#(#version_data),*]
  //   }
  // };
  // {
  //   // Generates the block data
  //   let mut f = File::create(&dir.join("data.rs"))?;
  //
  //   // Include macro must be one statement
  //   writeln!(f, "{{")?;
  //   for b in &latest.blocks {
  //     let name = b.name().to_case(Case::Pascal);
  //
  //     writeln!(f, "blocks.push(Data{{")?;
  //     writeln!(f, "  state: {},", b.id())?;
  //     writeln!(f, "  default_index: {},", b.default_index())?;
  //     writeln!(f, "  types: vec![")?;
  //     if b.states().is_empty() {
  //       writeln!(f, "    Type{{")?;
  //       writeln!(f, "      kind: Kind::{},", name)?;
  //       writeln!(f, "      state: {},", b.id())?;
  //       writeln!(f, "    }},")?;
  //     } else {
  //       for s in b.states() {
  //         writeln!(f, "    Type{{")?;
  //         writeln!(f, "      kind: Kind::{},", name)?;
  //         writeln!(f, "      state: {},", s.id())?;
  //         writeln!(f, "    }},")?;
  //       }
  //     }
  //     writeln!(f, "  ],")?;
  //     writeln!(f, "}});")?;
  //   }
  //   writeln!(f, "}}")?;
  // }
  // {
  //   // Generates the cross-versioning data
  //   //
  //   // This cannot be in a source file, as that would take multiple minutes
  // (and   // 10gb of ram) to compile. So we do a bit of pre-processing on
  // load.   let mut f = File::create(&dir.join("versions.csv"))?;
  //
  //   let mut to_old = vec![];
  //   for (i, v) in versions.iter().enumerate() {
  //     if i == 0 {
  //       continue;
  //     }
  //     if i >= versions.len() - 5 {
  //       // 1.8-1.12
  //       to_old.push(versions::generate_old(latest, v));
  //     } else {
  //       to_old.push(versions::generate(latest, v));
  //     }
  //   }
  //   for i in 0..to_old[0].len() {
  //     write!(f, "{},", i)?;
  //     for (j, arr) in to_old.iter().enumerate() {
  //       write!(f, "{}", arr[i])?;
  //       if j != to_old.len() - 1 {
  //         write!(f, ",")?;
  //       }
  //     }
  //     writeln!(f)?;
  //   }
  // }
  Ok(())
}

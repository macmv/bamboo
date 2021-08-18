mod block;
mod fixed;
mod paletted;
mod versions;

use crate::util;
pub use block::{Block, State};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fs,
  fs::File,
  io::Write,
  path::Path,
};

#[derive(Debug)]
struct BlockVersion {
  blocks: Vec<Block>,
  // Used to lookup block by name
  names:  HashMap<String, usize>,
}

impl BlockVersion {
  pub fn new() -> Self {
    BlockVersion { blocks: vec![], names: HashMap::new() }
  }
  pub fn add_block(&mut self, block: Block) {
    self.names.insert(block.name().to_string(), self.blocks.len());
    self.blocks.push(block);
  }
  pub fn get(&self, name: &str) -> Option<&Block> {
    Some(&self.blocks[*self.names.get(name)?])
  }
}

pub fn generate_kinds(dir: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
  let files = util::load_versions(dir, "blocks.json")?;

  let mut versions = vec![];
  for f in files {
    let fname = f.parent().unwrap().file_name().unwrap().to_str().unwrap();
    let version_id = fname.split('.').nth(1).unwrap().parse::<i32>()?;
    if version_id < 13 {
      versions.push(fixed::load_data(&fs::read_to_string(f)?)?);
    } else {
      versions.push(paletted::load_data(&fs::read_to_string(f)?)?);
    }
  }
  let latest = &versions[0];

  let mut kinds = HashSet::new();
  for b in &latest.blocks {
    kinds.insert(b.name().to_case(Case::Pascal));
  }
  Ok(kinds)
}

pub fn generate(dir: &Path) -> Result<TokenStream, Box<dyn Error>> {
  let files = util::load_versions(dir, "blocks.json")?;
  let dir = dir.join("block");

  let mut versions = vec![];
  for f in files {
    let fname = f.parent().unwrap().file_name().unwrap().to_str().unwrap();
    let version_id = fname.split('.').nth(1).unwrap().parse::<i32>()?;
    if version_id < 13 {
      versions.push(fixed::load_data(&fs::read_to_string(f)?)?);
    } else {
      versions.push(paletted::load_data(&fs::read_to_string(f)?)?);
    }
  }
  let latest = &versions[0];

  let mut kinds = vec![];
  for b in &latest.blocks {
    kinds.push(b.name().to_case(Case::Pascal));
  }
  let mut names = vec![];
  for b in &latest.blocks {
    names.push(b.name());
  }
  let name_len = latest.blocks.len();

  let mut block_data = vec![];
  for b in &latest.blocks {
    let state = b.id();
    let default_index = b.default_index();
    let name = b.name().to_case(Case::Pascal);

    let mut types = vec![];
    if b.states().is_empty() {
      types.push(quote!(Type{
        kind: Kind::#name,
        state: #state,
      }));
    } else {
      for s in b.states() {
        let sid = s.id();
        types.push(quote!(Type{
          kind: Kind::#name,
          state: #sid,
        }));
      }
    }

    let out = quote! {
      blocks.push(Data{
        state: #state,
        default_index: #default_index,
        types: vec![#(#types),*],
      });
    };
    block_data.push(out);
  }

  fs::create_dir_all(&dir)?;
  let out = quote! {
    /// Auto generated block kind. This is directly generated
    /// from prismarine data.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ToPrimitive, FromPrimitive)]
    pub enum Kind {
      #(#kinds),*
    }

    impl FromStr for Kind {
      type Err = InvalidBlock;
      fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
          #(#names => Ok(Self::#kinds)),*
          _ => Err(InvalidBlock(s.into())),
        }
      }
    }
    pub fn names() -> &'static [&'static str; #name_len] {
      &[#(#names),*]
    }

    /// Generates a table from all block kinds to any block data that kind has. This
    /// does not include cross-versioning data. This includes information like the
    /// block states, the properties it might have, and custom handlers for when the
    /// block is place (things like making fences connect, or making stairs rotate
    /// correctly).
    ///
    /// This should only be called once, and will be done internally in the
    /// [`WorldManager`](crate::world::WorldManager). This is left public as it may
    /// be moved to a seperate crate in the future, as it takes a long time to
    /// generate the source files for this.
    ///
    /// Most of this function is generated at compile time. See
    /// `gens/src/block/mod.rs` and `build.rs` for more.
    pub fn generate_kinds() -> Vec<Data> {
      let mut blocks = vec![];
      #(#block_data)*
      blocks
    }
  };
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
  Ok(out)
}

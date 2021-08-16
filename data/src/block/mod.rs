mod block;
mod fixed;
mod paletted;
mod versions;

use crate::util;
pub use block::{Block, State};
use convert_case::{Case, Casing};
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
  pub fn get(&self, name: &str) -> &Block {
    &self.blocks[self.names[name]]
  }
}

pub fn generate(dir: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
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

  fs::create_dir_all(&dir)?;
  let mut out = HashSet::new();
  {
    // Generates the block kinds enum
    let mut f = File::create(&dir.join("kind.rs"))?;
    writeln!(f, "/// Auto generated block kind. This is directly generated")?;
    writeln!(f, "/// from prismarine data.")?;
    writeln!(f, "#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ToPrimitive, FromPrimitive)]")?;
    writeln!(f, "pub enum Kind {{")?;
    for b in &latest.blocks {
      let name = b.name().to_case(Case::Pascal);
      writeln!(f, "  {},", name)?;
      out.insert(name);
    }
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "impl FromStr for Kind {{")?;
    writeln!(f, "  type Err = InvalidBlock;")?;
    writeln!(f)?;
    writeln!(f, "  fn from_str(s: &str) -> Result<Self, Self::Err> {{")?;
    writeln!(f, "    match s {{")?;
    for b in &latest.blocks {
      writeln!(f, "      \"{}\" => Ok(Self::{}),", b.name(), b.name().to_case(Case::Pascal))?;
    }
    writeln!(f, "      _ => Err(InvalidBlock(s.into())),")?;
    writeln!(f, "    }}")?;
    writeln!(f, "  }}")?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "pub fn names() -> &'static [&'static str; {}] {{", latest.blocks.len())?;
    writeln!(f, "  &[")?;
    for b in &latest.blocks {
      writeln!(f, "    \"{}\",", b.name())?;
    }
    writeln!(f, "  ]")?;
    writeln!(f, "}}")?;
  }
  {
    // Generates the block data
    let mut f = File::create(&dir.join("data.rs"))?;

    // Include macro must be one statement
    writeln!(f, "{{")?;
    for b in &latest.blocks {
      let name = b.name().to_case(Case::Pascal);

      writeln!(f, "blocks.push(Data{{")?;
      writeln!(f, "  state: {},", b.id())?;
      writeln!(f, "  default_index: {},", b.default_index())?;
      writeln!(f, "  types: vec![")?;
      if b.states().is_empty() {
        writeln!(f, "    Type{{")?;
        writeln!(f, "      kind: Kind::{},", name)?;
        writeln!(f, "      state: {},", b.id())?;
        writeln!(f, "    }},")?;
      } else {
        for s in b.states() {
          writeln!(f, "    Type{{")?;
          writeln!(f, "      kind: Kind::{},", name)?;
          writeln!(f, "      state: {},", s.id())?;
          writeln!(f, "    }},")?;
        }
      }
      writeln!(f, "  ],")?;
      writeln!(f, "}});")?;
    }
    writeln!(f, "}}")?;
  }
  {
    // Generates the cross-versioning data
    //
    // This cannot be in a source file, as that would take multiple minutes (and
    // 10gb of ram) to compile. So we do a bit of pre-processing on load.
    let mut f = File::create(&dir.join("versions.csv"))?;

    let mut to_old = vec![];
    for (i, v) in versions.iter().enumerate() {
      if i == 0 {
        continue;
      }
      if i >= versions.len() - 5 {
        // 1.8-1.12
        to_old.push(versions::generate_old(latest, v));
      } else {
        to_old.push(versions::generate(latest, v));
      }
    }
    for i in 0..to_old[0].len() {
      write!(f, "{},", i)?;
      for (j, arr) in to_old.iter().enumerate() {
        write!(f, "{}", arr[i])?;
        if j != to_old.len() - 1 {
          write!(f, ",")?;
        }
      }
      writeln!(f)?;
    }
  }
  Ok(out)
}

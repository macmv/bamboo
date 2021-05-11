mod fixed;
mod paletted;
mod versions;

use convert_case::{Case, Casing};
use std::{collections::HashMap, error::Error, fs, fs::File, io::Write, path::Path};

#[derive(Debug, PartialEq, Eq, Clone)]
struct State {
  // In fixed data, this is block id << 4 | meta
  // In paletted data, this is a global state id
  id:         u32,
  // All properties for this state. Empty on fixed states.
  properties: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct Block {
  // In fixed data, id is the block id << 4
  // In paletted data, this is the min state id
  id:            u32,
  // In fixed data, this is an array of all variations
  // In paletted data, this is an array of all states
  states:        Vec<State>,
  // Is 0 in fixed data
  // In paletted data, this is an index into states
  default_index: u32,
  // Always the full name of the block (for example, grass_block)
  name:          String,
}

#[derive(Debug)]
struct BlockVersion {
  blocks: Vec<Block>,
}

pub fn generate(dir: &Path) -> Result<(), Box<dyn Error>> {
  let dir = Path::new(dir).join("block");

  let versions = vec![
    paletted::load_data(include_str!("../../minecraft-data/data/pc/1.16.2/blocks.json"))?,
    paletted::load_data(include_str!("../../minecraft-data/data/pc/1.15.2/blocks.json"))?,
    paletted::load_data(include_str!("../../minecraft-data/data/pc/1.14.4/blocks.json"))?,
    // 1.13 is a seperate version, but the json is malformatted. So we only support
    // 1.13.2.
    paletted::load_data(include_str!("../../minecraft-data/data/pc/1.13.2/blocks.json"))?,
    fixed::load_data(include_str!("../../minecraft-data/data/pc/1.12/blocks.json"))?,
    fixed::load_data(include_str!("../../minecraft-data/data/pc/1.11/blocks.json"))?,
    fixed::load_data(include_str!("../../minecraft-data/data/pc/1.10/blocks.json"))?,
    fixed::load_data(include_str!("../../minecraft-data/data/pc/1.9/blocks.json"))?,
    fixed::load_data(include_str!("../../minecraft-data/data/pc/1.8/blocks.json"))?,
  ];
  let latest = &versions[0];

  fs::create_dir_all(&dir)?;
  {
    // Generates the block kinds enum
    let mut f = File::create(&dir.join("kind.rs"))?;
    writeln!(f, "/// Auto generated block kind. This is directly generated")?;
    writeln!(f, "/// from prismarine data.")?;
    writeln!(f, "#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]")?;
    writeln!(f, "pub enum Kind {{")?;
    for b in &latest.blocks {
      let name = b.name.to_case(Case::Pascal);
      writeln!(f, "  {},", name)?;
    }
    writeln!(f, "}}")?;
  }
  {
    // Generates the block data
    let mut f = File::create(&dir.join("data.rs"))?;

    // Include macro must be one statement
    writeln!(f, "{{")?;
    for b in &latest.blocks {
      let name = b.name.to_case(Case::Pascal);

      writeln!(f, "blocks.insert(Kind::{}, Data{{", name)?;
      writeln!(f, "  state: {},", b.id)?;
      writeln!(f, "  default_index: {},", b.default_index)?;
      writeln!(f, "  types: vec![")?;
      for s in &b.states {
        writeln!(f, "    Type{{")?;
        writeln!(f, "      kind: Kind::{},", name)?;
        writeln!(f, "      state: {},", s.id)?;
        writeln!(f, "    }},")?;
      }
      writeln!(f, "  ],")?;
      writeln!(f, "}});")?;
    }
    writeln!(f, "}}")?;
  }
  {
    // Generates the cross-versioning data
    let mut f = File::create(&dir.join("versions.rs"))?;

    // Include macro must be one statement
    writeln!(f, "{{")?;
    for v in &versions[1..] {
      let (to_old, to_new) = versions::generate(latest, v);

      write!(f, "to_old.push(vec![")?;
      for id in to_old {
        write!(f, "{}, ", id)?;
      }
      writeln!(f, "]);")?;

      writeln!(f, "let map = HashMap::new()")?;
      for (k, v) in to_new {
        writeln!(f, "map.insert({}, {});", k, v)?;
      }
      writeln!(f, "to_new.push(map);")?;
    }
    writeln!(f, "}}")?;
  }
  Ok(())
}

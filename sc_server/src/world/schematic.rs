use super::{CountedChunk, MultiChunk};
use crate::block;
use sc_common::{
  math::{ChunkPos, Pos},
  util::nbt::{ParseError, NBT},
};
use std::{collections::HashMap, fs, fs::File, io::Read, str::FromStr};

pub fn load_from_file(
  chunks: &mut HashMap<ChunkPos, CountedChunk>,
  path: &str,
  new_func: impl Fn() -> CountedChunk + Copy,
) -> Result<(), ParseError> {
  let mut f = File::open(path).expect("no file found");
  let metadata = fs::metadata(path).expect("unable to read metadata");
  let mut buf = vec![0; metadata.len() as usize];
  f.read(&mut buf).expect("file was too large");

  let tag = NBT::deserialize_file(buf)?;
  let compound = tag.compound();
  dbg!(&compound);
  let width: usize = compound["Width"].unwrap_short().try_into().unwrap();
  let length: usize = compound["Length"].unwrap_short().try_into().unwrap();
  let height: usize = compound["Height"].unwrap_short().try_into().unwrap();

  let material = tag.compound()["Materials"].unwrap_string();
  match material {
    "Alpha" => {
      let names: HashMap<i16, String> = compound["SchematicaMapping"]
        .unwrap_compound()
        .iter()
        .map(|(name, val)| {
          (val.unwrap_short(), convert_alpha_name(name.strip_prefix("minecraft:").unwrap().into()))
        })
        .collect();
      let blocks = tag.compound()["Blocks"].unwrap_byte_arr().to_vec();
      // TODO: `data` should be used! It should look something like this:
      // ```
      // let name = names[blocks[i]];
      // let v1_8_id = id_from_name(name) << 4 | data[i];
      // let new_id = convert_id(v1_8_id, ProtocolVersion::V1_8);
      // ```
      //
      // let data = tag.compound()["Data"].unwrap_byte_arr().to_vec();
      for y in 0..height {
        for z in 0..length {
          for x in 0..width {
            let i = ((y * height + z) * width) + x;
            let bid = blocks[i] as i16;
            let pos = Pos::new(x as i32, y as i32, z as i32);
            let name = &names[&bid];
            chunks
              .entry(pos.chunk())
              .or_insert_with(new_func)
              .chunk
              .lock()
              .set_kind(pos.chunk_rel(), block::Kind::from_str(name).unwrap())
              .unwrap();
          }
        }
      }
    }
    _ => panic!("invalid schematic file {}: unknown material {}", path, material),
  }

  Ok(())
}

fn convert_alpha_name(name: String) -> String {
  match name.as_str() {
    "grass" => "grass_block",
    _ => return name,
  }
  .into()
}

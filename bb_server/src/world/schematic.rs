use super::CountedChunk;
use crate::block;
use bb_common::{
  math::{ChunkPos, Pos},
  nbt::{ParseError, NBT},
  version::BlockVersion,
};
use std::{collections::HashMap, fs, fs::File, io::Read, str::FromStr};

pub fn load_from_file(
  chunks: &mut HashMap<ChunkPos, CountedChunk>,
  path: &str,
  types: &block::TypeConverter,
  new_func: impl Fn() -> CountedChunk + Copy,
) -> Result<(), ParseError> {
  let mut f = File::open(path).expect("no file found");
  let metadata = fs::metadata(path).expect("unable to read metadata");
  let mut buf = vec![0; metadata.len() as usize];
  f.read(&mut buf).expect("file was too large");

  let tag = NBT::deserialize_file(buf)?;
  let compound = tag.compound();
  // dbg!(&compound.keys());
  //
  // dbg!(&compound["Width"]);
  // dbg!(&compound["Length"]);
  // dbg!(&compound["Height"]);
  //
  // dbg!(&compound["WEOriginX"]);
  // dbg!(&compound["WEOriginY"]);
  // dbg!(&compound["WEOriginZ"]);
  //
  // dbg!(&compound["WEOffsetX"]);
  // dbg!(&compound["WEOffsetY"]);
  // dbg!(&compound["WEOffsetZ"]);
  //
  // dbg!(&compound["TileEntities"]);
  // dbg!(&compound["Entities"]);
  //
  // dbg!(&compound["Materials"]);
  // dbg!(&compound["Platform"]);
  // dbg!(&compound["Data"]);
  // dbg!(&compound["Blocks"]);

  let width: usize = compound["Width"].unwrap_short().try_into().unwrap();
  let length: usize = compound["Length"].unwrap_short().try_into().unwrap();
  let height: usize = compound["Height"].unwrap_short().try_into().unwrap();

  let material = tag.compound()["Materials"].unwrap_string();
  match material {
    "Alpha" => {
      if compound.contains_key("SchematicaMapping") {
        let names: HashMap<i16, String> = compound["SchematicaMapping"]
          .unwrap_compound()
          .iter()
          .map(|(name, val)| {
            (
              val.unwrap_short(),
              convert_alpha_name(name.strip_prefix("minecraft:").unwrap().into()),
            )
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
              let i = ((y * length + z) * width) + x;
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
      } else {
        // World edit
        let blocks = tag.compound()["Blocks"].unwrap_byte_arr().to_vec();
        // let data = tag.compound()["Data"].unwrap_byte_arr().to_vec();
        for y in 0..height {
          for z in 0..length {
            for x in 0..width {
              let i = ((y * length + z) * width) + x;
              let bid = (blocks[i] as u32) << 4;
              if bid == 0 {
                continue;
              }
              let pos = Pos::new(x as i32, y as i32, z as i32);
              chunks
                .entry(pos.chunk())
                .or_insert_with(new_func)
                .chunk
                .lock()
                .set_kind(pos.chunk_rel(), types.kind_from_id(bid, BlockVersion::V1_12))
                .unwrap();
            }
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

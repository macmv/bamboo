use super::CountedChunk;
use crate::block;
use bb_common::{
  math::{ChunkPos, Pos},
  nbt::{ParseError, WrongTag, NBT},
  version::BlockVersion,
};
use std::{collections::HashMap, fs, fs::File, io::Read, str::FromStr};

pub enum SchematicError {
  Parse(ParseError),
  WrongTag(WrongTag),
}

impl From<ParseError> for SchematicError {
  fn from(e: ParseError) -> Self { SchematicError::Parse(e) }
}
impl From<WrongTag> for SchematicError {
  fn from(e: WrongTag) -> Self { SchematicError::WrongTag(e) }
}

pub fn load_from_file(
  chunks: &mut HashMap<ChunkPos, CountedChunk>,
  path: &str,
  types: &block::TypeConverter,
  new_func: impl Fn() -> CountedChunk + Copy,
) -> Result<(), SchematicError> {
  let mut f = File::open(path).expect("no file found");
  let metadata = fs::metadata(path).expect("unable to read metadata");
  let mut buf = vec![0; metadata.len() as usize];
  f.read_exact(&mut buf).expect("file was too large");

  let tag = NBT::deserialize_file(buf)?;
  let compound = tag.compound().unwrap();
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

  let width: usize = compound["Width"].short()?.try_into().unwrap();
  let length: usize = compound["Length"].short()?.try_into().unwrap();
  let height: usize = compound["Height"].short()?.try_into().unwrap();

  let material = tag.compound().unwrap()["Materials"].string()?;
  match material {
    "Alpha" => {
      if compound.contains_key("SchematicaMapping") {
        let names: HashMap<i16, String> = compound["SchematicaMapping"]
          .compound()?
          .iter()
          .map(|(name, val)| {
            Ok((val.short()?, convert_alpha_name(name.strip_prefix("minecraft:").unwrap().into())))
          })
          .collect::<Result<_, WrongTag>>()?;
        let blocks = tag.compound().unwrap()["Blocks"].byte_arr()?.to_vec();
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
        let blocks = tag.compound().unwrap()["Blocks"].byte_arr()?.to_vec();
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

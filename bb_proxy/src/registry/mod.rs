use bb_common::{nbt, util::Buffer, version::ProtocolVersion};
use serde::Serialize;

mod biomes;
mod chat_type;
mod damage_type;
mod dimensions;

#[derive(Debug, Clone, Serialize)]
struct LoginInfo {
  #[serde(rename = "minecraft:dimension_type")]
  dimensions: Codec<dimensions::Dimension>,
  #[serde(rename = "minecraft:worldgen/biome")]
  biomes:     Codec<biomes::Biome>,
  #[serde(rename = "minecraft:chat_type")]
  chat:       Codec<chat_type::ChatType>,
  #[serde(rename = "minecraft:damage_type")]
  damage:     Codec<damage_type::DamageType>,
}
#[derive(Debug, Clone, Serialize)]
struct Codec<T> {
  #[serde(rename = "type")]
  ty:    String,
  value: Vec<CodecItem<T>>,
}
#[derive(Debug, Clone, Serialize)]
struct CodecItem<T> {
  name:    String,
  id:      i32,
  element: T,
}

pub fn write_single_dimension<T>(
  out: &mut Buffer<T>,
  _ver: ProtocolVersion,
  world_min_y: i32,
  world_height: u32,
) where
  std::io::Cursor<T>: std::io::Write,
{
  let dimension = dimensions::overworld(world_min_y, world_height);
  out.write_buf(&nbt::to_nbt("", &dimension).unwrap().serialize());
}

pub fn write_codec<T>(
  out: &mut Buffer<T>,
  ver: ProtocolVersion,
  world_min_y: i32,
  world_height: u32,
) where
  std::io::Cursor<T>: std::io::Write,
{
  let dimension = dimensions::overworld(world_min_y, world_height);
  let dimension_tag = nbt::to_nbt("", &dimension).unwrap();

  let info = LoginInfo {
    dimensions: Codec {
      ty:    "minecraft:dimension_type".into(),
      value: vec![CodecItem {
        name:    "minecraft:overworld".into(),
        id:      0,
        element: dimension,
      }],
    },
    biomes:     Codec { ty: "minecraft:worldgen/biome".into(), value: biomes::all() },
    chat:       Codec { ty: "minecraft:chat_type".into(), value: chat_type::all() },
    damage:     Codec { ty: "minecraft:damage_type".into(), value: damage_type::all() },
  };

  // Dimension codec
  out.write_buf(&nbt::to_nbt("", &info).unwrap().serialize());
  if ver >= ProtocolVersion::V1_19 {
    // Current dimension type (key in dimension codec)
    out.write_str("minecraft:overworld");
    // Current world
    out.write_str("minecraft:overworld");
  } else {
    // World codec (included in dimension)
    out.write_buf(&dimension_tag.serialize());
    // Current world
    out.write_str("minecraft:overworld");
  }
}

use bb_common::{nbt, util::Buffer, version::ProtocolVersion};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct Dimension {
  ambient_light:        f32,
  bed_works:            bool,
  coordinate_scale:     f32,
  effects:              String,
  has_ceiling:          bool,
  has_raids:            bool,
  has_skylight:         bool,
  height:               i32, // 1.17+
  infiniburn:           String,
  logical_height:       i32,
  min_y:                i32, // 1.17+
  natural:              bool,
  piglin_safe:          bool,
  fixed_time:           i64,
  respawn_anchor_works: bool,
  ultrawarm:            bool,

  // 1.19+
  monster_spawn_light_level:       i32,
  monster_spawn_block_light_limit: i32,
}

#[derive(Debug, Clone, Serialize)]
struct Biome {
  category:          String,
  depth:             f32,
  downfall:          f32,
  effects:           BiomeEffects,
  precipitation:     String,
  scale:             f32,
  temperature:       f32,
  has_precipitation: bool,
}
#[derive(Debug, Clone, Serialize)]
struct BiomeEffects {
  sky_color:       i32,
  fog_color:       i32,
  water_fog_color: i32,
  water_color:     i32,
  #[serde(skip_serializing_if = "Option::is_none")]
  foliage_color:   Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  grass_color:     Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  mood_sound:      Option<MoodSound>, // 1.18.2+
}
#[derive(Debug, Clone, Serialize)]
struct MoodSound {
  block_search_extent: i32,
  offset:              f64,
  sound:               String,
  tick_delay:          i32,
}

#[derive(Debug, Clone, Serialize)]
struct LoginInfo {
  #[serde(rename = "minecraft:dimension_type")]
  dimensions: Codec<Dimension>,
  #[serde(rename = "minecraft:worldgen/biome")]
  biomes:     Codec<Biome>,
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

fn single_dimension(world_min_y: i32, world_height: u32) -> Dimension {
  Dimension {
    piglin_safe:          false,
    natural:              true,
    ambient_light:        0.0,
    fixed_time:           6000,
    infiniburn:           "#minecraft:infiniburn_overworld".into(),
    respawn_anchor_works: false,
    has_skylight:         true,
    bed_works:            true,
    effects:              "minecraft:overworld".into(),
    has_raids:            false,
    logical_height:       128,
    coordinate_scale:     1.0,
    ultrawarm:            false,
    has_ceiling:          false,
    min_y:                world_min_y,
    height:               (world_height as i32 + 15) / 16 * 16,

    monster_spawn_light_level:       7,
    monster_spawn_block_light_limit: 7,
  }
}

pub fn write_single_dimension<T>(
  out: &mut Buffer<T>,
  _ver: ProtocolVersion,
  world_min_y: i32,
  world_height: u32,
) where
  std::io::Cursor<T>: std::io::Write,
{
  let dimension = single_dimension(world_min_y, world_height);
  out.write_buf(&nbt::to_nbt("", &dimension).unwrap().serialize());
}

pub fn write_dimensions<T>(
  out: &mut Buffer<T>,
  ver: ProtocolVersion,
  world_min_y: i32,
  world_height: u32,
) where
  std::io::Cursor<T>: std::io::Write,
{
  let dimension = single_dimension(world_min_y, world_height);
  let biome = Biome {
    precipitation:     "rain".into(),
    depth:             1.0,
    temperature:       1.0,
    scale:             1.0,
    downfall:          1.0,
    category:          "none".into(),
    has_precipitation: true,
    effects:           BiomeEffects {
      sky_color:       0x78a7ff,
      fog_color:       0xc0d8ff,
      water_fog_color: 0x050533,
      water_color:     0x3f76e4,
      foliage_color:   None,
      grass_color:     None,
      mood_sound:      Some(MoodSound {
        block_search_extent: 8,
        offset:              2.0,
        sound:               "minecraft:ambient.cave".into(),
        tick_delay:          6000,
      }),
      // sky_color:       0xff00ff,
      // water_color:     0xff00ff,
      // fog_color:       0xff00ff,
      // water_fog_color: 0xff00ff,
      // grass_color:     0xff00ff,
      // foliage_color:   0x00ffe5,
      // grass_color:     0xff5900,
    },
  };
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
    biomes:     Codec {
      ty:    "minecraft:worldgen/biome".into(),
      value: vec![CodecItem { name: "minecraft:plains".into(), id: 0, element: biome }],
    },
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

#[test]
fn test_codec() {
  use bb_common::nbt::Tag;

  let expected = Tag::new_compound(&[
    ("piglin_safe", Tag::Byte(0)),
    ("natural", Tag::Byte(1)),
    ("ambient_light", Tag::Float(0.0)),
    ("fixed_time", Tag::Long(6000)),
    ("infiniburn", Tag::String("".into())),
    ("respawn_anchor_works", Tag::Byte(0)),
    ("has_skylight", Tag::Byte(1)),
    ("bed_works", Tag::Byte(1)),
    ("effects", Tag::String("minecraft:overworld".into())),
    ("has_raids", Tag::Byte(0)),
    ("logical_height", Tag::Int(128)),
    ("coordinate_scale", Tag::Float(1.0)),
    ("ultrawarm", Tag::Byte(0)),
    ("has_ceiling", Tag::Byte(0)),
    // 1.17+
    ("min_y", Tag::Int(0)),
    ("height", Tag::Int(256)),
    // 1.19+
    ("monster_spawn_light_level", Tag::Int(7)),
    ("monster_spawn_block_light_limit", Tag::Int(7)),
  ]);
  let dimension = Dimension {
    piglin_safe:          false,
    natural:              true,
    ambient_light:        0.0,
    fixed_time:           6000,
    infiniburn:           "".into(),
    respawn_anchor_works: false,
    has_skylight:         true,
    bed_works:            true,
    effects:              "minecraft:overworld".into(),
    has_raids:            false,
    logical_height:       128,
    coordinate_scale:     1.0,
    ultrawarm:            false,
    has_ceiling:          false,
    min_y:                0,
    height:               256,

    monster_spawn_light_level:       7,
    monster_spawn_block_light_limit: 7,
  };
  assert_eq!(expected, nbt::to_tag(&dimension).unwrap());
  let expected = Tag::new_compound(&[
    ("precipitation", Tag::String("rain".into())),
    ("depth", Tag::Float(1.0)),
    ("temperature", Tag::Float(1.0)),
    ("scale", Tag::Float(1.0)),
    ("downfall", Tag::Float(1.0)),
    ("category", Tag::String("none".into())),
    ("has_precipitation", Tag::Bool(true)),
    (
      "effects",
      Tag::new_compound(&[
        ("sky_color", Tag::Int(0x78a7ff)),
        ("fog_color", Tag::Int(0xc0d8ff)),
        ("water_fog_color", Tag::Int(0x050533)),
        ("water_color", Tag::Int(0x3f76e4)),
        // ("sky_color", Tag::Int(0xff00ff)),
        // ("water_color", Tag::Int(0xff00ff)),
        // ("fog_color", Tag::Int(0xff00ff)),
        // ("water_fog_color", Tag::Int(0xff00ff)),
        // ("grass_color", Tag::Int(0xff00ff)),
        // ("foliage_color", Tag::Int(0x00ffe5)),
        // ("grass_color", Tag::Int(0xff5900)),
      ]),
    ),
  ]);
  let biome = Biome {
    precipitation:     "rain".into(),
    depth:             1.0,
    temperature:       1.0,
    scale:             1.0,
    downfall:          1.0,
    category:          "none".into(),
    has_precipitation: true,
    effects:           BiomeEffects {
      sky_color:       0x78a7ff,
      fog_color:       0xc0d8ff,
      water_fog_color: 0x050533,
      water_color:     0x3f76e4,
      foliage_color:   None,
      grass_color:     None,
      mood_sound:      None,
      // sky_color:       0xff00ff,
      // water_color:     0xff00ff,
      // fog_color:       0xff00ff,
      // water_fog_color: 0xff00ff,
      // grass_color:     0xff00ff,
      // foliage_color:   0x00ffe5,
      // grass_color:     0xff5900,
    },
  };
  dbg!(&expected);
  dbg!(&nbt::to_tag(&biome).unwrap());
  assert_eq!(expected, nbt::to_tag(&biome).unwrap());
}

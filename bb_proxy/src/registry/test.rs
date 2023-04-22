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

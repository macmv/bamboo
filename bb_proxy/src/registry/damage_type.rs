use super::CodecItem;
use serde::Serialize;

// IN_FIRE
// LIGHTNING_BOLT
// ON_FIRE
// LAVA
// HOT_FLOOR
// IN_WALL
// CRAMMING
// DROWN
// STARVE
// CACTUS
// FALL
// FLY_INTO_WALL
// OUT_OF_WORLD
// GENERIC
// MAGIC
// WITHER
// DRAGON_BREATH
// DRY_OUT
// SWEET_BERRY_BUSH
// FREEZE
// STALAGMITE

#[derive(Debug, Clone, Serialize)]
pub struct DamageType {
  exhaustion: f32,
  message_id: String,
  scaling:    String,
  #[serde(skip_serializing_if = "Option::is_none")]
  effects:    Option<String>,
}

pub(super) fn all() -> Vec<CodecItem<DamageType>> {
  vec![CodecItem {
    name:    "minecraft:in_fire".into(),
    id:      0,
    element: DamageType {
      exhaustion: 0.1,
      message_id: "inFire".into(),
      scaling:    "when_caused_by_living_non_player".into(),
      effects:    Some("burning".into()),
    },
  }]
}

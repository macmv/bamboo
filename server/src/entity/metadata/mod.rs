mod serialize;

use crate::item;
use common::{
  math::{BlockDirection, Pos},
  util::{Chat, UUID},
  version::ProtocolVersion,
};
use std::collections::HashMap;

pub struct VillagerData {
  ty:         i32,
  profession: i32,
  level:      i32,
}

pub enum Pose {
  Standing,
  FallFlying,
  Sleeping,
  Swimming,
  SpinAttack,
  Sneaking,
  Dying,
}

/// The types for each metadata field. Updated to the latest version of the
/// game.
pub enum Field {
  // Only valid on 1.8
  Short(i16),
  Int(i32),

  // Valid for all versions
  Byte(u8),
  Float(f32),
  String(String),
  Item(item::Stack),
  Position(Pos),
  Rotation(f32, f32, f32), // Rotation on x, y, z

  // Valid in 1.9+
  Varint(i32),
  Chat(Chat),
  Bool(bool),
  OptPosition(Option<Pos>),
  Direction(BlockDirection),
  OptUUID(Option<UUID>),
  OptBlockID(i32),

  // Valid for 1.13+
  OptChat(Option<Chat>),
  NBT(Vec<u8>),
  Particle(i32),

  // Valid for 1.14+
  VillagerData(VillagerData),
  OptVarint(Option<i32>),
  Pose(Pose),
}

pub struct Metadata {
  ver:    ProtocolVersion,
  // A sparse map of indices to serialized fields.
  fields: HashMap<u8, Field>,
}

impl Metadata {
  /// Creates a new entity metadata. This should not be called directly.
  /// Instead, use [`Entity::metadata()`](super::Entity::metadata).
  pub fn new(ver: ProtocolVersion) -> Self {
    Metadata { ver, fields: HashMap::new() }
  }
}

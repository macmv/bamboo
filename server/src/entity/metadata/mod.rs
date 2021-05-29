mod serialize;

use crate::item;
use common::{
  math::{BlockDirection, FPos},
  util::{Chat, UUID},
  version::ProtocolVersion,
};
use std::collections::HashMap;

pub struct VillagerData {
  ty:         i32,
  profession: i32,
  level:      i32,
}

/// The types for each metadata field. Updated to the latest version of the
/// game.
pub enum Field {
  // Only valid on 1.8
  Short(i16),
  Int(i32),

  // Valid in 1.9+
  Byte(u8),
  Varint(i32),
  Float(f32),
  String(String),
  Chat(Chat),
  OptChat(Option<Chat>),
  Slot(item::Stack),
  Bool(bool),
  Rotation(f32, f32),
  Position(FPos),
  OptPosition(Option<FPos>),
  Direction(BlockDirection),
  OptUUID(Option<UUID>),
  OptBlockID(i32),
  NBT(Vec<u8>),
  Particle(i32),
  VillagerData(VillagerData),
  OptVarint(Option<i32>),
  Pose(i32),
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

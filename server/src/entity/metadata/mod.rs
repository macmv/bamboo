mod serialize;

use std::collections::HashMap;

use common::version::ProtocolVersion;

/// The types for each metadata field. Updated to the latest version of the
/// game.
pub enum Type {
  Byte,
  Varint,
  Float,
  String,
  Chat,
  OptChat,
  Slot,
  Bool,
  Rotation,
  Position,
  OptPosition,
  Direction,
  OptUUID,
  OptBlockID,
  NBT,
  Particle,
  VaillagerData,
  OptVarint,
  Pose,
}

pub struct Metadata {
  ver:    ProtocolVersion,
  // A sparse map of indices to serialized fields.
  fields: HashMap<u8, Vec<u8>>,
}

impl Metadata {
  /// Creates a new entity metadata. This should not be called directly.
  /// Instead, use [`Entity::metadata()`](super::Entity::metadata).
  pub fn new(ver: ProtocolVersion) -> Self {
    Metadata { ver, fields: HashMap::new() }
  }
}
